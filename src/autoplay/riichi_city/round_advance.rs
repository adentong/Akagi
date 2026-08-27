//! Riichi City round advance: after each hand's settlement the client waits
//! on a "ready" (`req_user_prepare`) before dealing the next round. Majsoul
//! and Tenhou have no equivalent step, so this lives with the Riichi City
//! autoplay rather than in the platform-agnostic manager.
//!
//! The advance fires only after the client has *actually rendered* the
//! settlement OK button — sighted through the screen
//! ([`vision::capture_ok_button`]), polled once a second — so the press
//! never lands during the breakdown animation. No timed fallback: if the
//! button is never sighted, the server's own countdown advances the table.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast::error::RecvError;
use tokio::sync::RwLock;
use tokio::time::MissedTickBehavior;
use tracing::{debug, info, warn};

use crate::autoplay::inject::{InjectFrame, SharedInjectBus};
use crate::bridge::riichi_city::build;
use crate::config::{AppConfig, Platform};
use crate::event_bus::MjaiBus;
use crate::schema::MjaiEvent;

async fn autoplay_enabled_for_riichi(cfg: &Arc<RwLock<AppConfig>>) -> bool {
    let guard = cfg.read().await;
    guard.autoplay.enabled && guard.platform.kind == Platform::RiichiCity
}

/// Dedicated round-advance loop, on its own `MjaiBus` subscription so
/// end-of-hand plan backlogs cannot delay it. Advancing past the scoring
/// screen is one `req_user_prepare` per `EndKyoku`, sent only after the
/// client has actually rendered the OK button (sighted via
/// `vision::capture_ok_button`, once a second), held a short reading
/// beat. No timed fallback: if the button is never sighted, the server's
/// own countdown advances the table. `EndGame` needs nothing — the client
/// tears its end screens down when the next match's `cmd_enter_room`
/// arrives.
pub async fn round_advance_watcher(
    cfg: Arc<RwLock<AppConfig>>,
    inject: SharedInjectBus,
    bus: MjaiBus,
) {
    const POLL_INTERVAL: Duration = Duration::from_secs(1);
    /// Give up on sighting after this long — the client's own countdown is
    /// 59s, so anything past this means the window is gone or the detector
    /// is broken, and we must not fire blind.
    const SIGHTING_TIMEOUT: Duration = Duration::from_secs(90);

    let mut rx = bus.subscribe();
    loop {
        match rx.recv().await {
            Ok(ev) => match ev {
                MjaiEvent::EndKyoku => {
                    if !autoplay_enabled_for_riichi(&cfg).await {
                        continue;
                    }
                    let (_, yakus) = inject.settlement();
                    let mut ticker = tokio::time::interval(POLL_INTERVAL);
                    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
                    let deadline = tokio::time::Instant::now() + SIGHTING_TIMEOUT;
                    let mut sighted = false;
                    let mut aborted = false;
                    loop {
                        tokio::select! {
                            _ = ticker.tick() => {
                                if tokio::time::Instant::now() >= deadline {
                                    break;
                                }
                                let hit = tauri::async_runtime::spawn_blocking(
                                    crate::autoplay::riichi_city::vision::capture_ok_button,
                                )
                                .await
                                .ok()
                                .flatten();
                                if let Some(sight) = hit {
                                    debug!(
                                        pixels = sight.pixels,
                                        box = ?(sight.x0, sight.y0, sight.x1, sight.y1),
                                        "autoplay: OK button sighted"
                                    );
                                    sighted = true;
                                    break;
                                }
                            }
                            msg = rx.recv() => match msg {
                                // The round moved on without us (server
                                // countdown, manual click) — void cycle.
                                Ok(_) => {
                                    aborted = true;
                                    break;
                                }
                                Err(RecvError::Lagged(_)) => continue,
                                Err(RecvError::Closed) => return,
                            }
                        }
                    }
                    if aborted || !sighted {
                        if !aborted {
                            warn!(
                                "autoplay: OK button never sighted within {}s; \
                                 not advancing this round",
                                SIGHTING_TIMEOUT.as_secs()
                            );
                        }
                        continue;
                    }
                    tokio::time::sleep(Duration::from_millis(ok_button_beat_ms(yakus))).await;
                    info!("autoplay: sending round-advance (req_user_prepare)");
                    if !inject.send(InjectFrame {
                        gameplay: true,
                        bytes: build::user_prepare(),
                    }) {
                        warn!("autoplay: no injection relay for the round-advance press");
                    }
                }
                MjaiEvent::EndGame { .. } => {}
                _ => {}
            },
            Err(RecvError::Lagged(_)) => continue,
            Err(RecvError::Closed) => return,
        }
    }
}

/// Reading beat between sighting the OK button and pressing it: the button
/// only renders once the breakdown is up, so this is pure viewing time,
/// scaled by the number of yaku lines in the winning hand (each line past
/// two adds half a second). Draws count as zero yakus.
fn ok_button_beat_ms(yakus: u32) -> u64 {
    const BASE: u64 = 1_500;
    const CAP: u64 = 4_000;
    (BASE + u64::from(yakus.saturating_sub(2)) * 500).min(CAP)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The beat is a 1500ms base plus half a second per yaku line past
    /// two, capped at 4s — always after the button is visible, always
    /// short of the client's ~59s countdown by orders of magnitude.
    #[test]
    fn ok_button_beat_scales_with_yaku_count() {
        assert_eq!(ok_button_beat_ms(0), 1_500, "a draw reads the fastest");
        assert_eq!(ok_button_beat_ms(2), 1_500, "two lines are still base");
        assert_eq!(ok_button_beat_ms(3), 2_000);
        assert_eq!(ok_button_beat_ms(9), 4_000, "capped at 4s");
        assert_eq!(ok_button_beat_ms(20), 4_000);
    }
}

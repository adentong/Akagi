//! Mid-hand reconnect: rebuild the game state from the snapshot the
//! server attaches to `cmd_enter_room` (`is_reconnect: true`, non-null
//! `hand_status`).
//!
//! Without this, the tracker misses every event of the disconnect gap —
//! a riichi or meld declared then is invisible, and the bot plays the
//! rest of the hand on a wrong board. The snapshot carries everything
//! the client itself restores from (it rebuilds the whole table from
//! this one message): per-player rivers (`pai_he`), melds
//! (`fu_lou_list`), riichi (`is_li_zhi` + `li_zhi_pos`), scores
//! (`hand_chips`), our hand (`hand_cards`), the wall count
//! (`left_pai_shan`), and the dora indicators (`bao_pai_list`).
//!
//! The rebuild is a synthesized mjai event stream — `StartKyoku` with
//! the true scores, then per-player replay — so the engine derives
//! furiten/riichi/rivers itself instead of us poking its state. Two
//! conventions keep the stream byte-compatible with what live play
//! emits (and therefore with the engine's tolerances):
//! - Other players' draws are `Tsumo "?"` exactly as live hidden draws;
//!   ours use the discarded tile itself (the drawn tile's identity is
//!   unknown but immaterial — it entered and left the same turn), which
//!   keeps our tracked hand exact.
//! - Claimed tiles are absent from `pai_he` (the server removes them
//!   into the meld; the client re-appends them when restoring). Each
//!   claim is replayed as the source's `Tsumo`+`Dahai` of the claimed
//!   tile followed by the meld event, so rivers and the wall both come
//!   out right.
//!
//! Every synthesis is gated by the arithmetic the live captures
//! validated (5-for-5): the wall must equal `wall_base − (rivers +
//! claims)` and every hand size must match its expected value, with at
//! most the current actor holding a pending draw. Any mismatch refuses
//! the restore and the caller continues with the previous
//! (stale-state) behavior.

use serde_json::Value as JsonValue;

use super::consts::card_to_mjai;
use super::state::GameStatus;
use crate::schema::MjaiEvent;

/// Live-wall tiles at the start of a hand: 136 − 13n dealt − 14 dead.
fn wall_base(num_players: u8) -> i64 {
    match num_players {
        3 => 55,
        _ => 70,
    }
}

struct MeldSnap {
    /// `EMjActionType`: 2/3/4 chi variants, 5 pon, 6 daiminkan, 8 ankan,
    /// 9 kakan.
    action_type: i64,
    /// The claimed tile (the meld's `card`; for kakan, the added tile).
    card: u32,
    /// Tiles from the owner's hand.
    group: Vec<u32>,
    /// Kakan only: the pon tile that distinguishes the underlying pon.
    bu_gang: u32,
    /// `position` — absolute 0-based roster chair the claim came from.
    /// `None` for ankan.
    source_roster: Option<usize>,
}

struct PlayerSnap {
    actor: u8,
    score: i32,
    hand_n: i64,
    /// Our seat only (the server hides everyone else's).
    hand: Vec<u32>,
    river: Vec<u32>,
    melds: Vec<MeldSnap>,
    riichi: bool,
    riichi_pos: i64,
    kita: Vec<u32>,
}

/// Synthesize the event stream that rebuilds the tracker's state from a
/// mid-hand reconnect snapshot. `status` must already carry the
/// reconnect-resolved rotation and seat (see `on_enter_room`).
pub(crate) fn synthesise(data: &JsonValue, status: &GameStatus) -> Result<RestoreOutcome, String> {
    let n = status.num_players;
    let hs = data
        .get("hand_status")
        .filter(|h| !h.is_null())
        .ok_or("no hand_status")?;

    // ---- parse the per-player snapshot (payload order = roster order) ----
    let players_raw = data
        .get("players")
        .and_then(JsonValue::as_array)
        .filter(|p| p.len() == n as usize)
        .ok_or("players list missing or wrong length")?;
    let mut players: Vec<Option<PlayerSnap>> = Vec::with_capacity(n as usize);
    for p in players_raw {
        let uid = p
            .pointer("/user/user_id")
            .and_then(|v| {
                v.as_i64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .ok_or("player without user_id")?;
        let actor = status
            .actor_of(uid)
            .ok_or_else(|| format!("uid {uid} not in the rotated roster"))?;
        let melds = p
            .get("fu_lou_list")
            .and_then(JsonValue::as_array)
            .map(|list| {
                list.iter()
                    .map(|m| MeldSnap {
                        action_type: m
                            .get("action_type")
                            .and_then(JsonValue::as_i64)
                            .unwrap_or(0),
                        card: m.get("card").and_then(JsonValue::as_i64).unwrap_or(0) as u32,
                        group: m
                            .get("group_cards")
                            .and_then(JsonValue::as_array)
                            .map(|g| g.iter().map(|c| c.as_i64().unwrap_or(0) as u32).collect())
                            .unwrap_or_default(),
                        bu_gang: m
                            .get("bu_gang_peng_card")
                            .and_then(JsonValue::as_i64)
                            .unwrap_or(0) as u32,
                        source_roster: m
                            .get("position")
                            .and_then(JsonValue::as_i64)
                            .map(|p| p as usize),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let hand = p
            .get("hand_cards")
            .and_then(JsonValue::as_array)
            .map(|h| h.iter().map(|c| c.as_i64().unwrap_or(0) as u32).collect())
            .unwrap_or_default();
        let snap = PlayerSnap {
            actor,
            score: p.get("hand_chips").and_then(JsonValue::as_i64).unwrap_or(0) as i32,
            hand_n: p
                .get("hand_card_num")
                .and_then(JsonValue::as_i64)
                .unwrap_or(0),
            hand,
            river: p
                .get("pai_he")
                .and_then(JsonValue::as_array)
                .map(|r| r.iter().map(|c| c.as_i64().unwrap_or(0) as u32).collect())
                .unwrap_or_default(),
            melds,
            riichi: p
                .get("is_li_zhi")
                .and_then(JsonValue::as_bool)
                .unwrap_or(false),
            riichi_pos: p
                .get("li_zhi_pos")
                .and_then(JsonValue::as_i64)
                .unwrap_or(-1),
            kita: p
                .get("pull_north_list")
                .and_then(JsonValue::as_array)
                .map(|k| k.iter().map(|c| c.as_i64().unwrap_or(0) as u32).collect())
                .unwrap_or_default(),
        };
        while players.len() <= actor as usize {
            players.push(None);
        }
        players[actor as usize] = Some(snap);
    }
    let players: Vec<PlayerSnap> = players
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or("duplicate or missing seats after rotation")?;

    // ---- round identity ----
    let dealer_pos = hs
        .get("dealer_pos")
        .and_then(JsonValue::as_i64)
        .unwrap_or(0);
    let rel = (dealer_pos - status.shift).rem_euclid(n as i64) as u8;
    let bakaze = card_to_mjai(hs.get("quan_feng").and_then(JsonValue::as_i64).unwrap_or(0) as u32);
    let dora = hs
        .get("bao_pai_list")
        .and_then(JsonValue::as_array)
        .and_then(|d| d.first())
        .and_then(JsonValue::as_i64)
        .filter(|c| *c != 0)
        .ok_or("no dora indicator")? as u32;
    let wall_left = hs
        .get("left_pai_shan")
        .and_then(JsonValue::as_i64)
        .unwrap_or(-1);

    // ---- validation arithmetic (runs before the tehai so the pending
    // draw is known when our hand is reconstructed) ----
    // The full live-wall formula as validated on the captures (5-for-5):
    // draws = Σhand − 13n + Σrivers + meld-from-hand + kita + claims,
    // where meld-from-hand is 2 per chi/pon, 3 daiminkan, 4 ankan, 1
    // kakan, and each claim also consumed the source's draw of the
    // claimed tile.
    let meld_hand_total: i64 = players
        .iter()
        .map(|p| {
            p.melds
                .iter()
                .map(|m| match m.action_type {
                    2..=5 => 2,
                    6 => 3,
                    8 => 4,
                    9 => 1,
                    _ => 0,
                })
                .sum::<i64>()
                + p.kita.len() as i64
        })
        .sum();
    let n_claims: i64 = players
        .iter()
        .map(|p| {
            p.melds
                .iter()
                .filter(|m| matches!(m.action_type, 2 | 3 | 4 | 5 | 6 | 9))
                .count() as i64
        })
        .sum();
    let hand_total: i64 = players.iter().map(|p| p.hand_n).sum();
    let river_total: i64 = players.iter().map(|p| p.river.len() as i64).sum();
    let draws = hand_total - 13 * n as i64 + river_total + meld_hand_total + n_claims;
    if wall_base(n) - draws != wall_left {
        return Err(format!(
            "wall arithmetic mismatch: {} - {draws} draws != {wall_left}",
            wall_base(n)
        ));
    }
    let current_actor = data
        .pointer("/action_info/current_user_id")
        .and_then(JsonValue::as_i64)
        .and_then(|uid| status.actor_of(uid));
    let mut pending_draw: Option<u8> = None;
    for p in &players {
        // A chi/pon/daiminkan/kakan entry costs three tiles at rest: two
        // melded from hand plus the forced post-call discard that no draw
        // replenishes (validated on the live captures: a chi leaves 10,
        // two pons leave 7). An ankan costs four; each kita one. At most
        // the current actor may sit one above this (holding a draw).
        let cost: i64 = p
            .melds
            .iter()
            .map(|m| match m.action_type {
                8 => 4,
                _ => 3,
            })
            .sum::<i64>()
            + p.kita.len() as i64;
        let expected = 13i64 - cost;
        let delta = p.hand_n - expected;
        match delta {
            0 => {}
            1 if Some(p.actor) == current_actor => pending_draw = Some(p.actor),
            _ => {
                return Err(format!(
                    "actor {} hand is {} tiles, expected {expected}",
                    p.actor, p.hand_n
                ))
            }
        }
    }

    // ---- our tehai: current hand + own melds' from-hand tiles + kita,
    // minus the pending draw when we hold one ----
    let mut tehais: Vec<Vec<String>> = (0..n).map(|_| vec!["?".to_string(); 13]).collect();
    // The drawn tile when we are the pending actor: `zi_mo_cards` names
    // it when the server sends it; otherwise it is the last element of
    // `hand_cards` (validated live: a reconnect with a pending own draw
    // auto-tsumogiri'd exactly the last element). `None` while the
    // server still holds the tile hidden.
    let mut own_draw_tile: Option<u32> = None;
    {
        let me = &players[status.seat as usize];
        let mut hand = me.hand.clone();
        if pending_draw == Some(status.seat) {
            if hand.len() != 13 + 1 {
                return Err(format!(
                    "our hand_cards is {} tiles with a pending draw, not 14",
                    hand.len()
                ));
            }
            let draw = hand.pop().expect("len checked above");
            own_draw_tile = Some(draw);
        }
        let mut tehai: Vec<String> = hand.iter().map(|c| card_to_mjai(*c)).collect();
        for m in &me.melds {
            match m.action_type {
                2..=6 => tehai.extend(m.group.iter().map(|c| card_to_mjai(*c))),
                8 => {
                    tehai.extend(m.group.iter().map(|c| card_to_mjai(*c)));
                    tehai.push(card_to_mjai(m.card));
                }
                9 => {
                    tehai.extend(m.group.iter().map(|c| card_to_mjai(*c)));
                    tehai.push(card_to_mjai(m.card));
                }
                _ => return Err(format!("unknown meld action_type {}", m.action_type)),
            }
        }
        tehari_extend_kita(&mut tehai, &me.kita);
        if tehai.len() != 13 {
            return Err(format!(
                "our reconstructed tehai is {} tiles, not 13",
                tehai.len()
            ));
        }
        tehais[status.seat as usize] = tehai;
    }

    // ---- synthesize ----
    let mut events = Vec::new();
    events.push(MjaiEvent::StartKyoku {
        bakaze,
        dora_marker: card_to_mjai(dora),
        kyoku: rel + 1,
        honba: hs
            .get("ben_chang_num")
            .and_then(JsonValue::as_i64)
            .unwrap_or(0) as u8,
        kyotaku: hs
            .get("li_zhi_bang_num")
            .and_then(JsonValue::as_i64)
            .unwrap_or(0) as u8,
        oya: rel,
        scores: players.iter().map(|p| p.score).collect(),
        tehais,
        num_players: n,
    });

    let draw_of = |actor: u8, tile: u32| -> String {
        if actor == status.seat {
            card_to_mjai(tile)
        } else {
            "?".to_string()
        }
    };

    // Melds (each claim replays the source's discard of the claimed tile).
    for p in &players {
        for m in &p.melds {
            let source = match m.action_type {
                8 => None,
                _ => {
                    let roster_idx = m.source_roster.ok_or("meld without a source position")?;
                    let uid = players_raw
                        .get(roster_idx)
                        .and_then(|pj| {
                            pj.pointer("/user/user_id").and_then(|v| {
                                v.as_i64()
                                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                            })
                        })
                        .ok_or("meld source position out of range")?;
                    Some(
                        status
                            .actor_of(uid)
                            .ok_or("meld source uid not in the roster")?,
                    )
                }
            };
            if let Some(src) = source {
                let claimed = match m.action_type {
                    9 => m.bu_gang,
                    _ => m.card,
                };
                if claimed == 0 {
                    return Err("meld with a zero claimed tile".into());
                }
                events.push(MjaiEvent::Tsumo {
                    actor: src,
                    pai: draw_of(src, claimed),
                });
                events.push(MjaiEvent::Dahai {
                    actor: src,
                    pai: card_to_mjai(claimed),
                    tsumogiri: false,
                });
            }
            let target = source.unwrap_or(p.actor);
            match m.action_type {
                2..=4 => {
                    let [a, b] = pair(m)?;
                    events.push(MjaiEvent::Chi {
                        actor: p.actor,
                        target,
                        pai: card_to_mjai(m.card),
                        consumed: [a, b],
                    });
                }
                5 => {
                    let [a, b] = pair(m)?;
                    events.push(MjaiEvent::Pon {
                        actor: p.actor,
                        target,
                        pai: card_to_mjai(m.card),
                        consumed: [a, b],
                    });
                }
                6 => {
                    let [a, b, c] = triple(m)?;
                    events.push(MjaiEvent::Daiminkan {
                        actor: p.actor,
                        target,
                        pai: card_to_mjai(m.card),
                        consumed: [a, b, c],
                    });
                    rinshan_draw(&mut events, p.actor, status.seat);
                }
                8 => {
                    let mut consumed = m.group.iter().map(|c| card_to_mjai(*c)).collect::<Vec<_>>();
                    consumed.push(card_to_mjai(m.card));
                    if consumed.len() != 4 {
                        return Err("ankan without four tiles".into());
                    }
                    let [a, b, c, d] = [
                        consumed[0].clone(),
                        consumed[1].clone(),
                        consumed[2].clone(),
                        consumed[3].clone(),
                    ];
                    events.push(MjaiEvent::Ankan {
                        actor: p.actor,
                        consumed: [a, b, c, d],
                    });
                    rinshan_draw(&mut events, p.actor, status.seat);
                }
                9 => {
                    let [a, b] = pair(m)?;
                    events.push(MjaiEvent::Pon {
                        actor: p.actor,
                        target,
                        pai: card_to_mjai(m.bu_gang),
                        consumed: [a.clone(), b.clone()],
                    });
                    events.push(MjaiEvent::Kakan {
                        actor: p.actor,
                        pai: card_to_mjai(m.card),
                        consumed: [a, b, card_to_mjai(m.bu_gang)],
                    });
                    rinshan_draw(&mut events, p.actor, status.seat);
                }
                _ => return Err(format!("unknown meld action_type {}", m.action_type)),
            }
        }
        for _ in &p.kita {
            events.push(MjaiEvent::Kita {
                actor: p.actor,
                pai: Some("N".into()),
            });
            rinshan_draw(&mut events, p.actor, status.seat);
        }
    }

    // Rivers, with the riichi declaration inserted at its tile.
    for p in &players {
        for (i, tile) in p.river.iter().enumerate() {
            if p.riichi && p.riichi_pos == i as i64 {
                events.push(MjaiEvent::Reach {
                    actor: p.actor,
                    pai: None,
                });
            }
            events.push(MjaiEvent::Tsumo {
                actor: p.actor,
                pai: draw_of(p.actor, *tile),
            });
            events.push(MjaiEvent::Dahai {
                actor: p.actor,
                pai: card_to_mjai(*tile),
                tsumogiri: p.riichi && i as i64 > p.riichi_pos,
            });
        }
    }

    // The current actor's pending draw. For OTHER seats the tile stays
    // hidden (`?`). For OURS the snapshot's `hand_cards` names it (last
    // element), so it is emitted as itself — and the caller suppresses
    // the duplicate tsumo when the server re-offers the window live.
    let mut own_pending_draw = false;
    if let Some(actor) = pending_draw {
        if actor == status.seat {
            own_pending_draw = true;
            let pai = own_draw_tile
                .map(card_to_mjai)
                .unwrap_or_else(|| "?".into());
            events.push(MjaiEvent::Tsumo { actor, pai });
        } else {
            events.push(MjaiEvent::Tsumo {
                actor,
                pai: "?".to_string(),
            });
        }
    }

    Ok(RestoreOutcome {
        events,
        own_pending_draw,
    })
}

/// The synthesized stream plus the one piece of side-channel state the
/// bridge needs: whether WE held the pending draw (so it can suppress
/// the duplicate tsumo when the server re-offers the window).
pub(crate) struct RestoreOutcome {
    pub(crate) events: Vec<MjaiEvent>,
    pub(crate) own_pending_draw: bool,
}

fn tehari_extend_kita(tehai: &mut Vec<String>, kita: &[u32]) {
    for _ in kita {
        tehai.push("N".into());
    }
}

/// The two from-hand tiles of a chi/pon/kakan entry, as mjai strings.
fn pair(m: &MeldSnap) -> Result<[String; 2], String> {
    if m.group.len() < 2 {
        return Err("meld without its from-hand tiles".into());
    }
    Ok([card_to_mjai(m.group[0]), card_to_mjai(m.group[1])])
}

/// The three from-hand tiles of a daiminkan entry.
fn triple(m: &MeldSnap) -> Result<[String; 3], String> {
    if m.group.len() < 3 {
        return Err("daiminkan without its from-hand tiles".into());
    }
    Ok([
        card_to_mjai(m.group[0]),
        card_to_mjai(m.group[1]),
        card_to_mjai(m.group[2]),
    ])
}

/// A kan/kita replacement draw. Rinshan is dead-wall, so it never
/// touches the live-wall arithmetic; our own replacement is already
/// inside the snapshot hand, so only other seats get the event.
fn rinshan_draw(events: &mut Vec<MjaiEvent>, actor: u8, our_seat: u8) {
    if actor != our_seat {
        events.push(MjaiEvent::Tsumo {
            actor,
            pai: "?".into(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    /// The status Tier 0 produces for these fixtures: roster
    /// [1002, 1003, 1004, 1001(us)], initial_dealer_pos 1 → rotated
    /// player_list [1003, 1004, 1001, 1002], our seat 2, shift 1.
    fn status() -> GameStatus {
        GameStatus {
            player_list: vec![1003, 1004, 1001, 1002],
            seat: 2,
            shift: 1,
            num_players: 4,
            ..GameStatus::default()
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn player(
        uid: i64,
        score: i64,
        hand_n: i64,
        river: Value,
        hand: Value,
        melds: Value,
        riichi: bool,
        riichi_pos: i64,
    ) -> Value {
        json!({
            "user": { "user_id": uid, "nickname": "x" },
            "hand_chips": score,
            "hand_card_num": hand_n,
            "pai_he": river,
            "hand_cards": hand,
            "fu_lou_list": melds,
            "is_li_zhi": riichi,
            "li_zhi_pos": riichi_pos,
            "pull_north_list": [],
        })
    }

    fn plain(uid: i64, score: i64, hand_n: i64, river: Value) -> Value {
        player(uid, score, hand_n, river, Value::Null, json!([]), false, -1)
    }

    fn our_hand() -> Value {
        json!([23, 18, 25, 38, 1, 25, 2, 49, 37, 113, 20, 113, 18])
    }

    /// Live capture 2026-08-24 15:44:53 (East 1 late, wall 3, one chi
    /// each by two players — ids swapped for fakes, tile codes real).
    #[test]
    fn e1_two_chis_restores() {
        let data = json!({
            "is_reconnect": true,
            "initial_dealer_pos": 1,
            "hand_status": {
                "quan_feng": 49, "dealer_pos": 1, "ben_chang_num": 0,
                "li_zhi_bang_num": 0, "bao_pai_list": [1], "left_pai_shan": 3,
            },
            "action_info": { "current_user_id": 1003 },
            "players": [
                player(1002, 25000, 10,
                    json!([41,81,145,129,18,65,20,19,145,34,35,9,17,17,36,9]),
                    Value::Null,
                    json!([{ "action_type": 4, "card": 8, "group_cards": [6,7], "position": 3 }]),
                    false, -1),
                player(1003, 25000, 10,
                    json!([81,113,34,25,24,1,1,17,37,17,7,97,8,34,3,65,97,65]),
                    Value::Null,
                    json!([{ "action_type": 3, "card": 36, "group_cards": [35,37], "position": 0 }]),
                    false, -1),
                plain(1004, 25000, 13,
                    json!([113,8,6,145,38,4,18,3,35,38,19,22,81,1,19,18,65])),
                player(1001, 25000, 13,
                    json!([97,37,5,6,20,20,18,34,129,19,145,25,9,129,113,49]),
                    our_hand(), json!([]), false, -1),
            ],
        });
        let events = synthesise(&data, &status())
            .expect("restore should synthesise")
            .events;
        // StartKyoku + 2×(claim turn + meld) + 68 river pairs.
        assert_eq!(events.len(), 1 + 6 + 2 * 67);
        let MjaiEvent::StartKyoku {
            bakaze,
            kyoku,
            oya,
            honba,
            kyotaku,
            scores,
            tehais,
            ..
        } = &events[0]
        else {
            panic!("expected start_kyoku first");
        };
        assert_eq!(
            (bakaze.as_str(), *kyoku, *oya, *honba, *kyotaku),
            ("E", 1, 0, 0, 0)
        );
        assert_eq!(scores, &vec![25000; 4]);
        assert_eq!(tehais[2].len(), 13);
        assert!(tehais[2].iter().all(|t| t != "?"), "our tehai is real");
        assert!(tehais[0].iter().all(|t| t == "?"), "others stay hidden");

        // Melds are replayed in actor order: actor 0's chi first — it
        // claimed actor 3's 4m, and a hidden seat's draw is replayed
        // as "?".
        match &events[1..4] {
            [MjaiEvent::Tsumo {
                actor: 3,
                pai: tpai,
            }, _, MjaiEvent::Chi {
                actor: 0,
                target: 3,
                pai: cpai,
                ..
            }] => {
                assert_eq!(tpai, "?");
                assert_eq!(cpai, "4m");
            }
            other => panic!("expected the first claim turn, got {other:?}"),
        }
        // Then actor 3's chi: it claimed OUR 8p (source roster 3 → us,
        // actor 2), and our own draw is replayed as the tile itself.
        match &events[4..7] {
            [MjaiEvent::Tsumo {
                actor: 2,
                pai: tpai,
            }, MjaiEvent::Dahai {
                actor: 2,
                pai: dpai,
                ..
            }, MjaiEvent::Chi {
                actor: 3,
                target: 2,
                pai: cpai,
                consumed,
            }] => {
                assert_eq!(tpai, "8p");
                assert_eq!(dpai, "8p");
                assert_eq!(cpai, "8p");
                assert_eq!(consumed, &["6p".to_string(), "7p".to_string()]);
            }
            other => panic!("expected our claimed discard then the chi, got {other:?}"),
        }
    }

    /// Live capture 15:47:34 (East 2 honba 1, one riichi declared during
    /// the gap, one stick on the table).
    #[test]
    fn e2_riichi_marks_the_river() {
        let data = json!({
            "is_reconnect": true,
            "hand_status": {
                "quan_feng": 49, "dealer_pos": 2, "ben_chang_num": 1,
                "li_zhi_bang_num": 1, "bao_pai_list": [22], "left_pai_shan": 21,
            },
            "action_info": { "current_user_id": 1004 },
            "players": [
                player(1002, 25000, 13,
                    json!([25,49,65,145,2,129,37,24,40,34,277,33]),
                    Value::Null, json!([]), true, 9),
                plain(1003, 25000, 13, json!([97,113,145,25,49,40,1,49,2,34,25,21])),
                plain(1004, 25000, 13, json!([65,1,113,145,7,4,39,6,20,113,40,18,33])),
                player(1001, 25000, 13,
                    json!([97,49,129,25,8,97,65,81,65,22,24,23]),
                    our_hand(), json!([]), false, -1),
            ],
        });
        let events = synthesise(&data, &status())
            .expect("restore should synthesise")
            .events;
        let MjaiEvent::StartKyoku {
            kyoku,
            oya,
            honba,
            kyotaku,
            dora_marker,
            ..
        } = &events[0]
        else {
            panic!()
        };
        assert_eq!((*kyoku, *oya, *honba, *kyotaku), (2, 1, 1, 1));
        assert_eq!(dora_marker, "6s");

        // Actor 3 (uid 1002) declared riichi at river index 9 (2m); the
        // red 5s at index 10 is post-riichi.
        let reach_idx = events
            .iter()
            .position(|e| {
                matches!(
                    e,
                    MjaiEvent::Reach {
                        actor: 3,
                        pai: None
                    }
                )
            })
            .expect("a reach for actor 3");
        match &events[reach_idx + 1..reach_idx + 3] {
            [MjaiEvent::Tsumo { actor: 3, .. }, MjaiEvent::Dahai {
                actor: 3,
                pai,
                tsumogiri,
            }] => {
                assert_eq!(pai, "2m");
                assert!(
                    !*tsumogiri,
                    "the riichi tile itself is not marked tsumogiri"
                );
            }
            other => panic!("expected the riichi discard right after the reach, got {other:?}"),
        }
        match &events[reach_idx + 3..reach_idx + 5] {
            [MjaiEvent::Tsumo { actor: 3, .. }, MjaiEvent::Dahai { pai, tsumogiri, .. }] => {
                assert_eq!(pai, "5sr");
                assert!(*tsumogiri, "post-riichi discards are forced tsumogiri");
            }
            other => panic!("expected a post-riichi turn, got {other:?}"),
        }
    }

    /// Live capture 15:49:00 (East 2 honba 2 early, two pons by actor 1,
    /// the current actor holding a draw).
    #[test]
    fn e2_honba2_two_pons_and_held_draw() {
        let data = json!({
            "is_reconnect": true,
            "hand_status": {
                "quan_feng": 49, "dealer_pos": 2, "ben_chang_num": 2,
                "li_zhi_bang_num": 0, "bao_pai_list": [18], "left_pai_shan": 54,
            },
            "action_info": { "current_user_id": 1002 },
            "players": [
                plain(1002, 24000, 14, json!([97,4,34])),
                plain(1003, 25000, 13, json!([8,65])),
                player(1004, 35000, 7, json!([81,25,5,4,22]), Value::Null, json!([
                    { "action_type": 5, "card": 113, "group_cards": [113,113], "position": 0 },
                    { "action_type": 5, "card": 24, "group_cards": [24,24], "position": 1 },
                ]), false, -1),
                player(1001, 16000, 13, json!([37,9,6,6,19]),
                    our_hand(), json!([]), false, -1),
            ],
        });
        let events = synthesise(&data, &status())
            .expect("restore should synthesise")
            .events;
        let MjaiEvent::StartKyoku { honba, scores, .. } = &events[0] else {
            panic!()
        };
        assert_eq!(*honba, 2);
        assert_eq!(scores, &vec![25000, 35000, 16000, 24000]);

        // First pon: actor 1 claimed actor 3's white dragon.
        match &events[1..4] {
            [MjaiEvent::Tsumo {
                actor: 3,
                pai: tpai,
            }, MjaiEvent::Dahai {
                actor: 3,
                pai: dpai,
                ..
            }, MjaiEvent::Pon {
                actor: 1,
                target: 3,
                pai: ppai,
                consumed,
            }] => {
                assert_eq!(tpai, "?");
                assert_eq!(dpai, "P");
                assert_eq!(ppai, "P");
                assert_eq!(consumed, &["P".to_string(), "P".to_string()]);
            }
            other => panic!("expected the first pon with its claimed turn, got {other:?}"),
        }
        // Second pon: actor 1 claimed actor 0's 8s.
        match &events[4..7] {
            [_, _, MjaiEvent::Pon {
                actor: 1,
                target: 0,
                pai,
                ..
            }] => {
                assert_eq!(pai, "8s");
            }
            other => panic!("expected the second pon, got {other:?}"),
        }
        // The current actor (3) holds a draw — the last event.
        match events.last() {
            Some(MjaiEvent::Tsumo { actor: 3, pai }) => {
                assert_eq!(pai, "?");
            }
            other => panic!("expected the pending draw, got {other:?}"),
        }
    }

    /// Live capture 2026-08-26 19:33:01 — WE were the current actor
    /// holding a draw: `hand_card_num` 14 and `hand_cards` naming all 14
    /// tiles (ids swapped for fakes, tile codes real; wall 48 = 70 − 17
    /// rivers − 5 draws of the new hand). The first-cut restore refused
    /// this ("tehai is 14 tiles") and the whole post-reconnect game ran
    /// on a stale board. The tehai must drop the draw (the last element
    /// — the server's own timeout auto-tsumogiri discarded exactly it,
    /// card 113) and re-add it as our pending tsumo.
    #[test]
    fn own_pending_draw_restores() {
        let data = json!({
            "is_reconnect": true,
            "hand_status": {
                "quan_feng": 65, "dealer_pos": 2, "ben_chang_num": 0,
                "li_zhi_bang_num": 0, "bao_pai_list": [3], "left_pai_shan": 48,
            },
            "action_info": { "current_user_id": 1001 },
            "players": [
                plain(1002, 25000, 13, json!([9, 4, 34, 8, 65])),
                plain(1003, 25000, 13, json!([25, 8, 65, 5, 4])),
                plain(1004, 25000, 13, json!([34, 81, 7, 6, 5, 22])),
                player(1001, 27700, 14, json!([113, 97, 65, 17, 7]),
                    json!([35, 19, 36, 4, 40, 23, 20, 23, 18, 261, 34, 5, 38, 113]),
                    json!([]), false, -1),
            ],
        });
        let outcome = synthesise(&data, &status()).expect("restore should synthesise");
        assert!(outcome.own_pending_draw, "we held the draw");
        // Our tehai is the 13 tiles without the draw.
        let MjaiEvent::StartKyoku { tehais, .. } = &outcome.events[0] else {
            panic!()
        };
        assert_eq!(tehais[2].len(), 13);
        assert!(
            !tehais[2].contains(&"P".to_string()) || {
                // the draw was the LAST 113; one P remains in hand only if
                // hand_cards had two — it did not, so no P in the tehai.
                tehais[2].iter().filter(|t| t.as_str() == "P").count() == 0
            }
        );
        // The stream ends with our pending tsumo naming the dropped tile.
        match outcome.events.last() {
            Some(MjaiEvent::Tsumo { actor: 2, pai }) => assert_eq!(pai, "P"),
            other => panic!("expected our pending draw, got {other:?}"),
        }
    }

    /// A snapshot whose wall does not add up is refused — the caller
    /// falls back to stale-state continuation instead of emitting a
    /// wrong board.
    #[test]
    fn corrupted_wall_is_refused() {
        let data = json!({
            "is_reconnect": true,
            "hand_status": {
                "quan_feng": 49, "dealer_pos": 1, "ben_chang_num": 0,
                "li_zhi_bang_num": 0, "bao_pai_list": [1], "left_pai_shan": 4,
            },
            "action_info": { "current_user_id": 1003 },
            "players": [
                plain(1002, 25000, 13, json!([41,81])),
                plain(1003, 25000, 13, json!([81,113])),
                plain(1004, 25000, 13, json!([113,8])),
                player(1001, 25000, 13, json!([97,37]),
                    our_hand(), json!([]), false, -1),
            ],
        });
        let err = synthesise(&data, &status())
            .err()
            .expect("wall 4 does not fit 4 rivers");
        assert!(err.contains("wall"), "unexpected error: {err}");
    }
}

use serde::{Deserialize, Serialize};

/// Game platform whose traffic the proxy intercepts. Determines which
/// [`crate::bridge::Bridge`] is instantiated per WebSocket flow.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum Platform {
    #[default]
    Majsoul,
    Tenhou,
    /// Riichi City (麻雀一番街). Native client only (no web build), so it is
    /// captured exclusively through the MITM proxy — the Chromium/CDP backend
    /// does not apply. Observe-only (no autoplay).
    RiichiCity,
}

impl Platform {
    /// Short name used as a subdirectory under the log session
    /// (e.g. `<session>/<subdir>/<flow>.log`).
    pub fn subdir(self) -> &'static str {
        match self {
            Platform::Majsoul => "majsoul",
            Platform::Tenhou => "tenhou",
            Platform::RiichiCity => "riichi_city",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PlatformConfig {
    pub kind: Platform,
}

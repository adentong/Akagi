//! Shared schema types used across the project.
//!
//! Anything that needs to travel between modules — protocol events, IPC
//! payloads between backend and frontend, persisted records — lives here so
//! it isn't owned by any single subsystem.

pub mod inspector;
pub mod ipc;
pub mod mjai;

pub use inspector::{
    BotReaction, CaptureSource, FrameDirection, FrameRaw, HttpAnnotation, HttpBody, HttpExchange,
    HttpHeader, HttpPhase, InspectorEntry, ParsedFrame,
};
pub use ipc::{
    BotInfo, BotSettings, BotStatus, CaptureKind, CaptureStatus, LoadStage, LogEntry, Notification,
    NotifyLevel, Snapshot,
};
pub use mjai::{GameEndReason, MjaiEvent};

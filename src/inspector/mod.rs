//! Inspector data path — captures every WS frame, MJAI event, and bot
//! reaction into one canonical timeline.
//!
//! This module is the plumbing that gets pipeline events out of the proxy
//! / bridge / mjai bus / bot manager and into one place:
//!
//! - One file: `<session>/inspector.jsonl`. Each line is a serialized
//!   `schema::InspectorEntry`. The file is the only consumer — read it
//!   by hand when debugging a session.
//! - Single `InspectorWriter` cloned via `Arc` to every emitter
//!   (proxy/handler.rs, capture/chromium/cdp.rs, mjai_bus subscriber,
//!   bot manager). Writers serialize to disk synchronously.

pub mod annotate;

use crate::schema::InspectorEntry;
use anyhow::Result;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Append-only sink for inspector entries.
///
/// Cheap to clone — internally `Arc`-wrapped. The writer is shared
/// across the proxy handler, chromium capture, mjai-bus subscriber, and
/// bot manager; each calls `record(...)` independently.
#[derive(Clone)]
pub struct InspectorWriter {
    inner: Arc<Inner>,
}

struct Inner {
    file: Mutex<File>,
}

impl InspectorWriter {
    /// Open `<dir>/<file_name>` for append.
    pub fn open(path: &Path) -> Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            inner: Arc::new(Inner {
                file: Mutex::new(file),
            }),
        })
    }

    /// Record one entry. Best-effort: write failures are swallowed so
    /// emit-site code stays simple (the inspector is observability, not
    /// an authoritative store).
    pub fn record(&self, entry: InspectorEntry) {
        if let Ok(mut f) = self.inner.file.lock() {
            if serde_json::to_writer(&mut *f, &entry).is_ok() {
                let _ = f.write_all(b"\n");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{FrameDirection, FrameRaw, InspectorEntry};
    use tempfile::TempDir;

    fn sample_frame() -> InspectorEntry {
        InspectorEntry::WsFrame {
            ts_ms: 1,
            direction: FrameDirection::Down,
            flow_id: "test:1".into(),
            size: 4,
            raw: FrameRaw::Text("<Z/>".into()),
            parsed: None,
            emitted: 0,
        }
    }

    #[test]
    fn record_writes_disk() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("inspector.jsonl");
        let writer = InspectorWriter::open(&path).unwrap();
        let entry = sample_frame();
        writer.record(entry.clone());

        let body = std::fs::read_to_string(&path).unwrap();
        let line = body.lines().next().unwrap();
        let from_disk: InspectorEntry = serde_json::from_str(line).unwrap();
        assert_eq!(from_disk, entry);
    }

    #[test]
    fn record_appends_every_entry() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("inspector.jsonl");
        let writer = InspectorWriter::open(&path).unwrap();
        writer.record(sample_frame());
        writer.record(sample_frame());
        let body = std::fs::read_to_string(&path).unwrap();
        assert_eq!(body.lines().count(), 2);
    }
}

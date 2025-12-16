use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use std::time::{SystemTime, UNIX_EPOCH};

use common::Frame;
use crossbeam::channel::Sender;
use uuid::Uuid;

/// Represents a connected client with its communication channel and activity tracking.
#[derive(Clone)]
pub struct ClientEntry {
    pub client_id: Uuid,
    pub writer_sender: Sender<Arc<Frame>>,
    /// Last activity timestamp as milliseconds since UNIX epoch.
    /// Updated on every received message.
    last_activity_ms: Arc<AtomicU64>,
}

impl ClientEntry {
    pub fn new(client_id: Uuid, writer_sender: Sender<Arc<Frame>>) -> Self {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        
        Self {
            client_id,
            writer_sender,
            last_activity_ms: Arc::new(AtomicU64::new(now_ms)),
        }
    }

    /// Update the last activity timestamp to now.
    pub fn touch(&self) {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.last_activity_ms.store(now_ms, Ordering::Relaxed);
    }

    /// Get milliseconds since last activity.
    pub fn ms_since_last_activity(&self) -> u64 {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let last = self.last_activity_ms.load(Ordering::Relaxed);
        now_ms.saturating_sub(last)
    }

    /// Check if client is considered timed out.
    pub fn is_timed_out(&self, timeout_ms: u64) -> bool {
        self.ms_since_last_activity() > timeout_ms
    }
}

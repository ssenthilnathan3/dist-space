use std::sync::Arc;

use common::Frame;
use crossbeam::channel::Sender;
use uuid::Uuid;

#[derive(Clone)]
pub struct ClientEntry {
    pub client_id: Uuid,
    pub writer_sender: Sender<Arc<Frame>>,
}

impl ClientEntry {
    pub fn new(client_id: Uuid, writer_sender: Sender<Arc<Frame>>) -> Self {
        Self {
            client_id,
            writer_sender,
        }
    }
}

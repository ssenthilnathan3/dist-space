use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Frame {
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn total_len(&self) -> usize {
        4 + self.payload.len()
    }

    pub fn new_arc(payload: Vec<u8>) -> Arc<Frame> {
        Arc::new(Frame { payload })
    }
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum FrameError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Peer disconnected")]
    Disconnected,

    #[error("Payload too large: {0} bytes (max: {1})")]
    PayloadTooLarge(usize, usize),

    #[error("Protocol error: {0}")]
    Protocol(String),
}

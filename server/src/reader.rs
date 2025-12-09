// reader/src/lib.rs
use std::io::Read;
use std::net::TcpStream;
use std::sync::Arc;
use std::thread;

use common::error::FrameError;
use common::frame::Frame;
use common::protocol::ServerMessage;

use crate::ClientEntry;
use crate::state::ServerState;
use uuid::Uuid;

pub struct Reader;

type BroadcastFn =
    fn(origin_id: Uuid, frame: Arc<Frame>, clients: Arc<std::sync::Mutex<Vec<Arc<ClientEntry>>>>);

impl Reader {
    /// Reads exactly one length-prefixed frame from the stream.
    /// Returns Arc<Frame> for zero-copy broadcast.
    pub fn read_frame(stream: &mut TcpStream) -> Result<Arc<Frame>, FrameError> {
        const MAX_PAYLOAD_SIZE: usize = 1024 * 1024; // 1MB

        // Read prefix (length)
        let mut prefix = [0u8; 4];
        stream.read_exact(&mut prefix).map_err(|e| {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                FrameError::Disconnected
            } else {
                FrameError::Io(e)
            }
        })?;

        let length = u32::from_be_bytes(prefix) as usize;

        // Handle zero-length payload as valid (not error)
        if length == 0 {
            return Ok(Frame::new_arc(Vec::new()));
        }

        // Check payload size limit
        if length > MAX_PAYLOAD_SIZE {
            return Err(FrameError::PayloadTooLarge(length, MAX_PAYLOAD_SIZE));
        }

        // Read payload
        let mut payload = vec![0u8; length];
        stream.read_exact(&mut payload).map_err(|e| {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                FrameError::Disconnected
            } else {
                FrameError::Io(e)
            }
        })?;

        // Return Arc<Frame> without storing the prefix
        Ok(Frame::new_arc(payload))
    }

    /// Spawns a reader thread for a client connection
    /// Returns join handle for the thread
    pub fn spawn_reader_thread(
        stream: TcpStream,
        client_id: Uuid,
        state: Arc<ServerState>,
        broadcast_fn: BroadcastFn,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            Reader::run_reader_loop(stream, client_id, state, broadcast_fn);
        })
    }

    /// Main reader loop - handles all frames for a client until disconnect
    fn run_reader_loop(
        mut stream: TcpStream,
        client_id: Uuid,
        state: Arc<ServerState>,
        broadcast_fn: BroadcastFn,
    ) {
        let peer_addr = match stream.peer_addr() {
            Ok(addr) => addr,
            Err(_) => {
                eprintln!("[{}] Failed to get peer address", client_id);
                // Remove client from clients list
                state.remove_client(client_id);
                return;
            }
        };

        println!("[{}] Reader thread started for {}", client_id, peer_addr);

        loop {
            match Reader::read_frame(&mut stream) {
                Ok(frame) => {
                    match ServerMessage::decode(&frame.payload) {
                        Ok(ServerMessage::Operation(op)) => {
                            println!("[{}] Received Operation from client", client_id);

                            match ServerState::send_applied_op(&state, op) {
                                Ok(frame) => {
                                    broadcast_fn(
                                        client_id,
                                        frame,
                                        Arc::clone(&state.get_clients_arc()),
                                    );
                                }
                                Err(e) => {
                                    eprintln!(
                                        "[{}] Error applying operation for: {}",
                                        client_id, e
                                    );
                                }
                            }
                        }
                        Ok(ServerMessage::SyncDocument(_)) => {
                            // Handle SyncDocument if needed
                        }
                        Err(e) => {
                            eprintln!("[{}] Failed to decode message: {}", client_id, e);
                        }
                    }
                }
                Err(FrameError::Disconnected) => {
                    println!("[{}] Client disconnected: {}", client_id, peer_addr);
                    break;
                }
                Err(FrameError::PayloadTooLarge(size, max)) => {
                    eprintln!(
                        "[{}] Payload too large: {} > {} - disconnecting",
                        client_id, size, max
                    );
                    break;
                }
                Err(e) => {
                    eprintln!("[{}] Read error: {} - disconnecting", client_id, e);
                    break;
                }
            }
        }

        // Cleanup: remove client from clients list
        state.remove_client(client_id);
        println!("[{}] Reader thread exiting", client_id);
    }
}

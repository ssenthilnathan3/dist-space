use std::{
    io::{self, BufReader, Read, Write},
    net::TcpStream,
    sync::{Arc, Mutex},
    thread,
};

use common::{protocol::ServerMessage, space::OperationProto};

pub struct ClientState {
    pub client_id: String,
    pub doc_id: String,
    pub version: u64,
    pub buffer: String,
}

fn main() -> io::Result<()> {
    let client_id = uuid::Uuid::new_v4().to_string();
    let state = Arc::new(Mutex::new(ClientState {
        client_id,
        doc_id: String::new(),
        version: 0,
        buffer: String::new(),
    }));

    let mut stream: Option<TcpStream> = None;
    let stdin = io::stdin();

    println!("Test Client Ready");
    println!("Commands: CONNECT <host:port>, SEND <text>, EXIT");

    loop {
        let mut input = String::new();
        print!("> ");
        io::stdout().flush()?;
        stdin.read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        match parts[0].to_uppercase().as_str() {
            "CONNECT" => {
                if parts.len() != 2 {
                    println!("Usage: CONNECT <host:port>");
                    continue;
                }

                // Close existing connection if any
                if stream.is_some() {
                    println!("Closing existing connection");
                    stream = None;
                }

                match TcpStream::connect(parts[1]) {
                    Ok(new_stream) => {
                        let stream_clone = new_stream.try_clone()?;

                        // Reset state
                        {
                            let mut state_guard = state.lock().unwrap();
                            *state_guard = ClientState {
                                client_id: uuid::Uuid::new_v4().to_string(),
                                doc_id: String::new(),
                                version: 0,
                                buffer: String::new(),
                            };
                        }

                        // Spawn reader thread
                        let state_for_reader = Arc::clone(&state);
                        thread::spawn(move || {
                            if let Err(e) = reader_loop(new_stream, state_for_reader) {
                                eprintln!("Reader thread error: {}", e);
                            }
                        });

                        stream = Some(stream_clone);
                        println!("Connected to {}", parts[1]);
                    }
                    Err(e) => {
                        eprintln!("Failed to connect: {}", e);
                    }
                }
            }
            "SEND" => {
                if parts.len() != 2 {
                    println!("Usage: SEND <text>");
                    continue;
                }

                let text = parts[1];

                // Get current state for doc_id and version
                let (doc_id, version, client_id, buffer_len) = {
                    let state_guard = state.lock().unwrap();
                    (
                        state_guard.doc_id.clone(),
                        state_guard.version,
                        state_guard.client_id.clone(),
                        state_guard.buffer.len(),
                    )
                };

                if doc_id.is_empty() {
                    println!("Error: Not synchronized with any document yet");
                    continue;
                }

                // Send operation
                if let Some(ref mut s) = stream {
                    use common::space::{ReplaceOp, operation_proto::Kind};

                    let op_kind = Kind::Replace(ReplaceOp {
                        start: 0,
                        end: buffer_len as u32,
                        text: text.to_string(),
                        client_id: client_id.clone(),
                        client_version: version,
                    });

                    let operation = ServerMessage::Operation(OperationProto {
                        op_id: uuid::Uuid::new_v4().as_u64_pair().0,
                        kind: Some(op_kind),
                        doc_id,
                        client_id,
                        client_version: version,
                        server_version: 0,
                        new_content: text.to_string(),
                    });

                    let message = ServerMessage::encode(&operation);
                    let len_bytes = (message.len() as u32).to_be_bytes();

                    s.write_all(&len_bytes)?;
                    s.write_all(&message)?;
                    s.flush()?;

                    println!("OP_SENT");
                } else {
                    println!("Error: Not connected to any server");
                }
            }
            "EXIT" => {
                println!("Closing connection and exiting");
                break;
            }
            _ => {
                println!("Unknown command: {}", parts[0]);
                println!("Available: CONNECT, SEND, EXIT");
            }
        }
    }

    Ok(())
}

fn reader_loop(stream: TcpStream, state: Arc<Mutex<ClientState>>) -> io::Result<()> {
    let mut reader = BufReader::new(stream);

    loop {
        // Read length prefix
        let mut len_bytes = [0u8; 4];
        if reader.read_exact(&mut len_bytes).is_err() {
            break;
        }
        let payload_length = u32::from_be_bytes(len_bytes) as usize;
        println!("[DEBUG] Received payload length: {}", payload_length);

        // Read payload
        let mut payload_buffer = vec![0u8; payload_length];
        if reader.read_exact(&mut payload_buffer).is_err() {
            break;
        }
        println!(
            "[DEBUG] Received payload bytes: {:?}",
            &payload_buffer[..std::cmp::min(20, payload_length)]
        );

        // Decode ServerMessage
        match ServerMessage::decode(&*payload_buffer) {
            Ok(message) => {
                match message {
                    ServerMessage::Operation(_) => {
                        println!("[DEBUG] Decoded as Operation");
                    }
                    ServerMessage::SyncDocument(doc) => {
                        println!("[DEBUG] Decoded as SyncDocument");
                        // Update local state
                        {
                            let mut state_guard = state.lock().unwrap();
                            state_guard.doc_id = doc.doc_id.clone();
                            state_guard.version = doc.version;
                            state_guard.buffer = doc.content.clone();
                        }

                        // Print SYNC message
                        println!(
                            "SYNC {{ version: {}, doc_id: \"{}\", content: \"{}\" }}",
                            doc.version, doc.doc_id, doc.content
                        );
                    }
                    ServerMessage::Ping(seq) => {
                        println!("[DEBUG] Received Ping({})", seq);
                        // In a full implementation, we'd respond with Pong here
                    }
                    ServerMessage::Pong(seq) => {
                        println!("[DEBUG] Received Pong({})", seq);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to decode message: {}", e);
                eprintln!(
                    "[DEBUG] First 20 bytes of payload: {:?}",
                    &payload_buffer[..std::cmp::min(20, payload_length)]
                );
            }
        }
    }

    Ok(())
}

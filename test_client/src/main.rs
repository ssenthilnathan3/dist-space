use prost::Message;
use std::{
    io::{self, BufReader, Read, Write},
    net::TcpStream,
    sync::{Arc, Mutex},
    thread,
};

use common::{protocol::ServerMessage, workspace::OperationProto};

pub struct ClientState {
    pub doc_id: String,
    pub version: u32,
    pub buffer: String,
}

fn main() -> io::Result<()> {
    let state = Arc::new(Mutex::new(ClientState {
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
                let (doc_id, version) = {
                    let state_guard = state.lock().unwrap();
                    (state_guard.doc_id.clone(), state_guard.version)
                };

                if doc_id.is_empty() {
                    println!("Error: Not synchronized with any document yet");
                    continue;
                }

                // Send operation
                if let Some(ref mut s) = stream {
                    let operation = ServerMessage::Operation(OperationProto {
                        doc_id,
                        new_content: text.to_string(),
                        client_version: version,
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
            break; // Connection closed
        }
        let payload_length = u32::from_be_bytes(len_bytes) as usize;
        println!("[DEBUG] Received payload length: {}", payload_length);

        // Read payload
        let mut payload_buffer = vec![0u8; payload_length];
        if reader.read_exact(&mut payload_buffer).is_err() {
            break; // Connection closed
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

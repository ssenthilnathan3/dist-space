use std::{
    io::{self, BufReader, Read, Write},
    net::TcpStream,
    process,
    sync::{Arc, Mutex},
    thread,
};

use common::{
    protocol::ServerMessage,
    space::{OperationProto, ReplaceOp, operation_proto::Kind},
};
use uuid::Uuid;

use crate::types::ClientState;

mod types;

fn main() {
    let stream = TcpStream::connect("127.0.0.1:8000");

    // Use SyncDocumentProto instead of Document for shared state
    let client_id = Uuid::new_v4().to_string();
    let state = Arc::new(Mutex::new(ClientState {
        client_id,
        doc_id: String::new(),
        version: 0,
        buffer: String::new(),
    }));

    let state_clone = Arc::clone(&state);

    match stream {
        Ok(stream) => {
            let stream_clone = match stream.try_clone() {
                Ok(stream) => stream,
                Err(e) => {
                    eprintln!("Failed to clone stream: {}", e);
                    return;
                }
            };

            // Spawn reader thread
            thread::spawn(move || {
                if let Err(e) = reader_loop(stream, state_clone) {
                    eprintln!("\nReader thread error: {}", e);
                    eprintln!("Exiting application due to socket error.");
                    process::exit(1);
                }
            });

            // Run CLI loop in main thread
            if let Err(e) = cli_loop(stream_clone, Arc::clone(&state)) {
                eprintln!("CLI loop error: {}", e);
            }
        }
        Err(e) => {
            eprintln!("Failed to connect to server: {}", e)
        }
    }
}

fn reader_loop(stream: TcpStream, state: Arc<Mutex<ClientState>>) -> io::Result<()> {
    let mut reader = BufReader::new(stream);

    loop {
        // Read 4 bytes (big-endian u32) -> N (payload length)
        let mut len_bytes = [0u8; 4];
        reader.read_exact(&mut len_bytes)?;
        let payload_length = u32::from_be_bytes(len_bytes) as usize;

        // Read exactly N bytes -> payload
        let mut payload_buffer = vec![0u8; payload_length];
        reader.read_exact(&mut payload_buffer)?;

            match ServerMessage::decode(&*payload_buffer) {
            Ok(message) => match message {
                ServerMessage::Operation(_) => {
                    println!("Received an Operation message.");
                }
                ServerMessage::SyncDocument(doc) => {
                    println!("Received a SyncDocument message.");

                    // Update shared state
                    let mut current_state = state.lock().unwrap();
                    current_state.buffer = doc.content.clone();
                    current_state.version = doc.version;

                    // Store doc_id upon initial sync
                    if current_state.doc_id.is_empty() && !doc.doc_id.is_empty() {
                        current_state.doc_id = doc.doc_id.clone();
                    }

                    // Print short summary
                    let content_preview = doc.content.chars().take(80).collect::<String>();
                    println!(
                        "\n[SYNC] version={} doc_id={} content='{}...'",
                        doc.version, doc.doc_id, content_preview
                    );

                    print!("\nEnter command (put/send/quit): ");
                    io::stdout().flush()?;
                }
                ServerMessage::Ping(seq) => {
                    // Server is checking if we're alive - respond with Pong
                    // Note: We'd need access to the write stream here to respond
                    // For now, just log it. The proper solution is to share the writer
                    // between threads or use a channel.
                    println!("[Heartbeat] Received ping #{}", seq);
                }
                ServerMessage::Pong(_seq) => {
                    // We sent a ping (unusual for client), server responded
                    // Just ignore
                }
            },
            Err(e) => {
                eprintln!("\nFailed to decode protobuf message: {}", e);
            }
        }
    }
}

fn cli_loop(mut stream: TcpStream, state: Arc<Mutex<ClientState>>) -> io::Result<()> {
    let stdin = io::stdin();
    let mut command_buffer = String::new();

    loop {
        command_buffer.clear();
        print!("\nEnter command (put/send/quit): ");
        io::stdout().flush()?;
        stdin.read_line(&mut command_buffer)?;
        let command = command_buffer.trim();

        match command {
            "quit" => {
                println!("Closing socket and exiting.");
                break;
            }
            "put" | "send" => {
                // Lock state to read doc_id and version
                let current_state = state.lock().unwrap();
                let doc_id = current_state.doc_id.clone();
                let client_version = current_state.version;
                let client_id = current_state.client_id.clone();
                let current_buffer_len = current_state.buffer.len();
                drop(current_state); // Unlock state quickly

                if doc_id.is_empty() {
                    println!("Cannot 'put' yet. Awaiting initial SyncDocument from server...");
                    continue;
                }

                // Prompt user for new text
                println!("Enter full new document text (press Enter twice to finish input):");
                let mut new_content = String::new();
                loop {
                    let mut line = String::new();
                    stdin.read_line(&mut line)?;
                    if line.trim().is_empty() {
                        break;
                    }
                    new_content.push_str(&line);
                }

                let op_kind = Kind::Replace(ReplaceOp {
                    start: 0,
                    end: current_buffer_len as u32,
                    text: new_content.clone(),
                    client_id: client_id.clone(),
                    client_version,
                });

                let operation = OperationProto {
                    op_id: Uuid::new_v4().as_u64_pair().0,
                    kind: Some(op_kind),
                    doc_id,
                    client_id,
                    client_version,
                    server_version: 0,
                    new_content,
                };
                // Create ServerMessage containing the operation
                let server_message = ServerMessage::Operation(operation);
                let encoded = server_message.encode();
                let len_bytes = (encoded.len() as u32).to_be_bytes();

                // Send bytes to server
                stream.write_all(&len_bytes)?;
                stream.write_all(&encoded)?;
                stream.flush()?;

                println!(
                    "Sent Operation to server. Waiting for server confirmation (SyncDocument update)..."
                );
            }
            _ => {
                println!("Unknown command: {}", command);
            }
        }
    }
    Ok(())
}

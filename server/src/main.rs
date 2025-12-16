mod broadcaster;
mod client_entry;
mod reader;
mod state;
mod transform;
mod writer;

use common::protocol::ServerMessage;
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use common::Frame;
use common::proto::space::SyncDocumentProto;
use uuid::Uuid;

use crate::broadcaster::broadcast;
use crate::client_entry::ClientEntry;
use crate::reader::Reader;
use crate::state::{ServerState, MAX_CLIENTS, HEARTBEAT_INTERVAL_MS};
use crate::writer::Writer;

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8000")?;
    // Wrap the server state in an Arc *once* outside the loop.
    let server_state_arc = Arc::new(ServerState::new());
    
    println!("═══════════════════════════════════════════════════════════");
    println!("  Dist-Space Server v0.1.0");
    println!("  Listening on 127.0.0.1:8000");
    println!("  Max clients: {}", MAX_CLIENTS);
    println!("  Heartbeat interval: {}ms", HEARTBEAT_INTERVAL_MS);
    println!("═══════════════════════════════════════════════════════════");

    // Spawn heartbeat monitoring thread
    let heartbeat_state = Arc::clone(&server_state_arc);
    thread::spawn(move || {
        run_heartbeat_loop(heartbeat_state);
    });

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let peer_addr = stream.peer_addr().unwrap();
                println!("\n[Server] New connection: {}", peer_addr);

                // Check connection limit before proceeding
                if server_state_arc.client_count() >= MAX_CLIENTS {
                    eprintln!(
                        "[Server] Connection rejected: max clients ({}) reached",
                        MAX_CLIENTS
                    );
                    // Let the stream drop, closing the connection
                    continue;
                }

                // Generate new client_id for incoming connection
                let client_id = Uuid::new_v4();

                // Create a bounded channel
                let (tx, rx) = crossbeam::channel::bounded::<Arc<Frame>>(32);

                // Clone the stream for the writer thread
                let stream_writer = stream.try_clone()?;

                // Get the authoritative Document type
                let document = Arc::clone(&server_state_arc.get_document());

                // Lock the document to access its fields
                let (doc_id, content, version) = {
                    let doc_guard = document.lock().unwrap();
                    (
                        doc_guard.uuid.to_string(),
                        doc_guard.content.clone(),
                        doc_guard.version,
                    )
                };

                // Construct a new SyncDocument based on ServerMessage enum
                let server_message = ServerMessage::SyncDocument(SyncDocumentProto {
                    doc_id,
                    content,
                    version,
                });

                // Encode SyncDocument proto to Frame
                let frame = ServerMessage::encode(&server_message);

                // Spawn writer thread with its dedicated stream handle
                let _ = Writer::spawn_writer_thread(client_id, stream_writer, rx);

                // Immediately send a frame to the writer channel
                tx.send(Frame::new_arc(frame))
                    .expect("Failed to send frame to writer thread");

                // Create a new client_entry
                let client_entry = ClientEntry::new(client_id, tx);

                // Add client_entry to server state
                match server_state_arc.add_client(client_entry) {
                    Ok(()) => {
                        println!(
                            "[Server] Client {} registered (total: {})",
                            client_id,
                            server_state_arc.client_count()
                        );
                    }
                    Err(e) => {
                        eprintln!("[Server] Failed to add client: {}", e);
                        continue;
                    }
                }

                let state_clone = Arc::clone(&server_state_arc);

                let _ = Reader::spawn_reader_thread(stream, client_id, state_clone, broadcast);
            }
            Err(e) => {
                eprintln!("[Server] Connection failed: {}", e);
            }
        }
    }
    Ok(())
}

/// Heartbeat monitoring loop.
/// Periodically sends pings to all clients and removes timed-out clients.
fn run_heartbeat_loop(state: Arc<ServerState>) {
    let ping_sequence = AtomicU64::new(0);
    
    println!("[Heartbeat] Monitoring thread started");
    
    loop {
        thread::sleep(Duration::from_millis(HEARTBEAT_INTERVAL_MS));
        
        // Get next ping sequence number
        let seq = ping_sequence.fetch_add(1, Ordering::Relaxed);
        
        // Remove timed-out clients
        let removed = state.remove_timed_out_clients();
        if removed > 0 {
            println!("[Heartbeat] Removed {} timed-out client(s)", removed);
        }
        
        // Send ping to all remaining clients
        let pinged = state.send_ping_to_all(seq);
        if pinged > 0 {
            println!("[Heartbeat] Sent ping #{} to {} client(s)", seq, pinged);
        }
    }
}

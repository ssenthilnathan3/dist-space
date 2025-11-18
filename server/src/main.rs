mod broadcaster;
mod client_entry;
mod reader;
mod state;
mod writer;

use common::protocol::ServerMessage;
use std::net::TcpListener;
use std::sync::Arc;

use common::Frame;
use common::proto::workspace::SyncDocumentProto;
use uuid::Uuid;

use crate::broadcaster::broadcast;
use crate::client_entry::ClientEntry;
use crate::reader::Reader;
use crate::state::ServerState;
use crate::writer::Writer;

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8000")?;
    // Wrap the server state in an Arc *once* outside the loop.
    let server_state_arc = Arc::new(ServerState::new());
    println!("Server listening on port 80");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("New connection: {}", stream.peer_addr().unwrap());

                // Generate new client_id for incoming connection
                let client_id = Uuid::new_v4();

                // Create a bounded channel
                let (tx, rx) = crossbeam::channel::bounded::<Arc<Frame>>(32);

                // Clone the stream for the writer thread
                let stream_writer = stream.try_clone()?;

                // Get the authoritative Document typef
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
                server_state_arc
                    .add_client(client_entry)
                    .expect("Failed to add client to server state");

                let state_clone = Arc::clone(&server_state_arc);

                let _ = Reader::spawn_reader_thread(stream, client_id, state_clone, broadcast);
            }
            Err(e) => {
                eprintln!("Connection failed: {}", e);
            }
        }
    }
    Ok(())
}

// NOTE: Swapped `Vec` for `HashMap<ClientId, ...>` to ensure
// stable client positions/keys during additions/removals.
// This is necessary for future features like replay, checkpointing,
// or version vectors that rely on persistent client IDs and data stability.
// The transport layer is currently unaffected as it does not depend on order.

use std::sync::{Arc, Mutex};

use common::{
    Document, Frame,
    operation::{Operation, OperationLog},
    protocol::ServerMessage,
    space::{OperationProto, SyncDocumentProto},
};
use uuid::Uuid;

use crate::client_entry::ClientEntry;

/// Default document path for Phase 1 (single-document mode).
/// Will be replaced by dynamic file paths in Phase 2 (VFS).
#[allow(dead_code)]
const DEFAULT_DOC_PATH: &str = "main.txt";

/// Maximum number of concurrent client connections.
/// Protects against denial-of-service attacks.
pub const MAX_CLIENTS: usize = 100;

/// Client timeout in milliseconds (30 seconds).
/// Clients that don't respond to heartbeats within this window are disconnected.
pub const CLIENT_TIMEOUT_MS: u64 = 30_000;

/// Heartbeat interval in milliseconds (10 seconds).
/// Server sends ping to clients at this interval.
pub const HEARTBEAT_INTERVAL_MS: u64 = 10_000;

pub struct ServerState {
    clients: Arc<Mutex<Vec<Arc<ClientEntry>>>>,
    /// The default document for single-document mode (Phase 1).
    /// In Phase 2, this will be replaced by `workspace: Arc<Mutex<Workspace>>`
    /// with a HashMap<Path, Document> structure.
    document: Arc<Mutex<Document>>,
    op_log: Arc<OperationLog>,
}

impl ServerState {
    pub fn new() -> Self {
        let doc_id = Uuid::new_v4();
        Self {
            clients: Arc::new(Mutex::new(Vec::new())),
            document: Arc::new(Mutex::new(Document {
                uuid: doc_id,
                content: String::new(),
                version: 0,
            })),
            op_log: Arc::new(OperationLog::new()),
        }
    }

    pub fn get_document(&self) -> Arc<Mutex<Document>> {
        Arc::clone(&self.document)
    }

    /// Add a new client to the server state.
    /// Returns Err if the maximum client limit is reached.
    pub fn add_client(&self, client: ClientEntry) -> Result<(), String> {
        let mut clients = match self.clients.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                eprintln!("WARNING: Mutex was poisoned. Data might be in an inconsistent state.");
                poisoned.into_inner()
            }
        };

        // Check connection limit
        if clients.len() >= MAX_CLIENTS {
            return Err(format!(
                "Connection limit reached: {} clients already connected",
                MAX_CLIENTS
            ));
        }

        clients.push(Arc::new(client));
        println!(
            "[ServerState] Client added. Total clients: {}",
            clients.len()
        );

        Ok(())
    }

    /// Get the current number of connected clients.
    pub fn client_count(&self) -> usize {
        match self.clients.lock() {
            Ok(guard) => guard.len(),
            Err(poisoned) => poisoned.into_inner().len(),
        }
    }

    pub fn remove_client(&self, client_id: Uuid) -> Option<Arc<ClientEntry>> {
        let mut clients = match self.clients.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                eprintln!("WARNING: Mutex was poisoned. Data might be in an inconsistent state.");
                poisoned.into_inner()
            }
        };

        if let Some(pos) = clients.iter().position(|c| c.client_id == client_id) {
            let removed = clients.remove(pos);
            println!(
                "[ServerState] Client {} removed. Remaining clients: {}",
                client_id,
                clients.len()
            );
            Some(removed)
        } else {
            None
        }
    }

    /// Remove all clients that have timed out.
    /// Returns the number of clients removed.
    pub fn remove_timed_out_clients(&self) -> usize {
        let mut clients = match self.clients.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                eprintln!("WARNING: Mutex was poisoned. Recovering.");
                poisoned.into_inner()
            }
        };

        let before = clients.len();
        clients.retain(|client| {
            let timed_out = client.is_timed_out(CLIENT_TIMEOUT_MS);
            if timed_out {
                println!(
                    "[ServerState] Client {} timed out ({}ms since last activity)",
                    client.client_id,
                    client.ms_since_last_activity()
                );
            }
            !timed_out
        });

        before - clients.len()
    }

    /// Send a ping to all connected clients.
    /// Returns the number of clients pinged.
    pub fn send_ping_to_all(&self, sequence: u64) -> usize {
        let clients = match self.clients.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        };

        let ping_msg = ServerMessage::Ping(sequence);
        let ping_frame = Frame::new_arc(ServerMessage::encode(&ping_msg));

        let mut pinged = 0;
        for client in clients.iter() {
            if client.writer_sender.try_send(Arc::clone(&ping_frame)).is_ok() {
                pinged += 1;
            }
        }

        pinged
    }

    /// Update last activity time for a client.
    pub fn touch_client(&self, client_id: Uuid) {
        let clients = match self.clients.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        if let Some(client) = clients.iter().find(|c| c.client_id == client_id) {
            client.touch();
        }
    }

    pub fn append_op_log(&self, op: Operation) -> Result<(), String> {
        OperationLog::append_log_arc(Arc::clone(&self.op_log), op)
    }

    pub fn get_clients_arc(&self) -> Arc<Mutex<Vec<Arc<ClientEntry>>>> {
        Arc::clone(&self.clients)
    }

    pub fn send_applied_op(
        &self,
        operation_proto: OperationProto,
    ) -> Result<Arc<Frame>, std::io::Error> {
        let doc_mutex = self.get_document();

        if operation_proto.doc_id.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Operation missing doc_id",
            ));
        }

        let parsed_client_id = Uuid::parse_str(&operation_proto.client_id).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid client UUID")
        })?;

        let mut op_kind =
            Operation::convert_operation(operation_proto.clone()).ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "Missing op kind")
            })?;

        let client_version = operation_proto.client_version;

        let (updated_content, new_version) = {
            let mut doc = doc_mutex.lock().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to lock document: {}", e),
                )
            })?;

            if client_version > doc.version {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "Client version {} is from the future (server is {})",
                        client_version, doc.version
                    ),
                ));
            }

            if client_version < doc.version {
                // Get ops from log: [client_version, doc.version)
                let past_ops = self
                    .op_log
                    .get_ops_in_range(client_version, doc.version)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

                // Transform incoming op against all past ops
                for past_op in past_ops {
                    op_kind = crate::transform::transform(op_kind, past_op.kind);
                }
            }

            // Apply transformed op
            doc.apply_op(&op_kind)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

            (doc.content.clone(), doc.version)
        };

        // Log the operation
        // server_version is the version this op was applied TO (i.e., new_version - 1)
        let final_op = Operation {
            op_id: operation_proto.op_id,
            kind: op_kind,
            doc_id: operation_proto.doc_id.clone(),
            new_content: String::new(),
            client_id: parsed_client_id,
            client_version,
            server_version: new_version - 1,
        };

        if let Err(e) = self.append_op_log(final_op) {
            eprintln!("Failed to append to op_log: {}", e);
        }

        let sync_doc = SyncDocumentProto {
            doc_id: operation_proto.doc_id.clone(),
            content: updated_content,
            version: new_version,
        };

        let server_message = ServerMessage::SyncDocument(sync_doc);
        Ok(Frame::new_arc(ServerMessage::encode(&server_message)))
    }
}

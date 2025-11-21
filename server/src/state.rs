// NOTE (Future Devs): Swapped `Vec` for `HashMap<ClientId, ...>` to ensure
// stable client positions/keys during additions/removals.
// This is necessary for future features like replay, checkpointing,
// or version vectors that rely on persistent client IDs and data stability.
// The transport layer is currently unaffected as it does not depend on order.

use std::sync::{Arc, Mutex};

use common::{
    Document, Frame,
    protocol::ServerMessage,
    types::{Operation, OperationKind, OperationLog},
    workspace::{OperationProto, SyncDocumentProto},
};
use uuid::Uuid;

use crate::client_entry::ClientEntry;

pub struct ServerState {
    clients: Arc<Mutex<Vec<Arc<ClientEntry>>>>,
    document: Arc<Mutex<Document>>,
    op_log: Arc<OperationLog>,
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(Vec::new())),
            document: Arc::new(Mutex::new(Document {
                uuid: Uuid::new_v4(),
                content: String::new(),
                version: 0,
            })),
            op_log: Arc::new(OperationLog::new()),
        }
    }

    pub fn get_document(&self) -> Arc<Mutex<Document>> {
        Arc::clone(&self.document)
    }

    // TODO: use a generic “AddClientError” enum
    pub fn add_client(&self, client: ClientEntry) -> Result<(), String> {
        let mut clients = match self.clients.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                eprintln!("WARNING: Mutex was poisoned. Data might be in an inconsistent state.");
                poisoned.into_inner()
            }
        };

        clients.push(Arc::new(client));

        Ok(())
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
            Some(clients.remove(pos))
        } else {
            None
        }
    }

    pub fn append_op_log(&self, op: Operation) -> Result<(), String> {
        OperationLog::append_log_arc(Arc::clone(&self.op_log), op)
    }

    // pub fn get_clients(&self) -> Vec<Arc<ClientEntry>> {
    //     let clients = match self.clients.lock() {
    //         Ok(guard) => guard,
    //         Err(poisoned) => {
    //             // Log a severe error that the data might be inconsistent
    //             eprintln!("WARNING: Mutex was poisoned. Data might be in an inconsistent state.");
    //             poisoned.into_inner()
    //         }
    //     };

    //     clients.clone()
    // }
    //

    pub fn get_clients_arc(&self) -> Arc<Mutex<Vec<Arc<ClientEntry>>>> {
        Arc::clone(&self.clients)
    }

    // pub fn client_count(&self) -> usize {
    //     let clients = match self.clients.lock() {
    //         Ok(guard) => guard,
    //         Err(poisoned) => {
    //             // Log a severe error that the data might be inconsistent
    //             eprintln!("WARNING: Mutex was poisoned. Data might be in an inconsistent state.");
    //             poisoned.into_inner()
    //         }
    //     };

    //     clients.len()
    // }

    // pub fn find_client(&self, client_id: Uuid) -> Option<Arc<ClientEntry>> {
    //     let clients = match self.clients.lock() {
    //         Ok(guard) => guard,
    //         Err(poisoned) => {
    //             // Log a severe error that the data might be inconsistent
    //             eprintln!("WARNING: Mutex was poisoned. Data might be in an inconsistent state.");
    //             poisoned.into_inner()
    //         }
    //     };

    //     clients.iter().find(|c| c.client_id == client_id).cloned()
    // }
    //
    //

    pub fn send_applied_op(&self, operation: OperationProto) -> Result<Arc<Frame>, std::io::Error> {
        let doc_mutex = self.get_document();
        // 1. Parse and validate proto op
        if operation.doc_id.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Operation missing doc_id",
            ));
        }
        if operation.new_content.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Operation content is empty",
            ));
        }

        let op_kind: OperationKind = Operation::convert_operation(operation.clone())
            .expect("Operation kind was missing in the proto, which shouldn't happen here!");

        let parsed_client_id: Uuid =
            Uuid::parse_str(&operation.client_id).expect("Client ID is not a valid uuid");

        let operation = Operation {
            op_id: operation.op_id,
            kind: op_kind,
            client_id: parsed_client_id,
            server_version: operation.server_version,
            doc_id: operation.doc_id,
            new_content: operation.new_content,
            client_version: operation.client_version,
        };

        let (updated_content, new_version) = {
            let mut doc_guard = doc_mutex.lock().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to lock document: {}", e),
                )
            })?;

            doc_guard
                .apply_operation(operation.new_content.to_string(), operation.client_version)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
        };

        // Append op to op_log
        if let Err(e) = self.append_op_log(operation.clone()) {
            eprintln!("Failed to append to op_log: {}", e);
        }

        // Create SyncDocumentProto
        let sync_doc = SyncDocumentProto {
            doc_id: operation.doc_id.clone(),
            content: updated_content,
            version: new_version,
        };

        // Wrap in ServerMessage::SyncDocument
        let server_message = ServerMessage::SyncDocument(sync_doc);

        // Encode into payload
        let payload = ServerMessage::encode(&server_message);

        // Return Arc<Frame> to caller
        Ok(Frame::new_arc(payload))
    }
}

// NOTE: Swapped `Vec` for `HashMap<ClientId, ...>` to ensure
// stable client positions/keys during additions/removals.
// This is necessary for future features like replay, checkpointing,
// or version vectors that rely on persistent client IDs and data stability.
// The transport layer is currently unaffected as it does not depend on order.

use std::sync::{Arc, Mutex};

use common::{
    Document, Frame,
    protocol::ServerMessage,
    types::{Operation, OperationLog},
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
            client_version: client_version,
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

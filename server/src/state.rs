// NOTE (Future Devs): Swapped `Vec` for `HashMap<ClientId, ...>` to ensure
// stable client positions/keys during additions/removals.
// This is necessary for future features like replay, checkpointing,
// or version vectors that rely on persistent client IDs and data stability.
// The transport layer is currently unaffected as it does not depend on order.

use std::sync::{Arc, Mutex};

use common::{
    Document,
    types::{Operation, OperationLog},
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
}

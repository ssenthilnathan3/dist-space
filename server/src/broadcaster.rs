use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use common::Frame;
use crossbeam::channel::TrySendError;
use uuid::Uuid;

use crate::client_entry::ClientEntry;

pub fn broadcast(origin_id: Uuid, frame: Arc<Frame>, clients: Arc<Mutex<Vec<Arc<ClientEntry>>>>) {
    let mut failed_clients: HashSet<Uuid> = HashSet::new();
    let clients_snapshot: Vec<Arc<ClientEntry>>;

    {
        let clients_guard = clients.lock().unwrap();
        clients_snapshot = clients_guard.clone();
    }

    for client_entry in clients_snapshot {
        // If client_id is a String representing ip:port, this breaks:
        //      - clients reconnect with different port → treated as new client
        //      - NAT → port changes
        //      - proxies → unpredictable
        //      - ephemeral ports → randomness

        if client_entry.client_id != origin_id {
            let sender = &client_entry.writer_sender;

            match sender.try_send(Arc::clone(&frame)) {
                Ok(()) => println!("Message sent!"),

                Err(TrySendError::Full(_)) => {
                    // A slow client must not affect the performance of the rest of the system;
                    // any client whose writer channel is full is immediately dropped.
                    failed_clients.insert(client_entry.client_id.clone());
                }

                Err(TrySendError::Disconnected(_)) => {
                    failed_clients.insert(client_entry.client_id.clone());
                }
            }
        }
    }

    if !failed_clients.is_empty() {
        let mut clients_guard = clients.lock().unwrap();

        clients_guard.retain(|client_entry| {
            if failed_clients.contains(&client_entry.client_id) {
                println!("Removing disconnected client: {}", client_entry.client_id);
                false
            } else {
                true
            }
        });
    }
}

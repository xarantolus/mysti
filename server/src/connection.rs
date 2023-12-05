use std::sync::atomic::AtomicU64;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Clone)]
enum BroadcastMessage {
    Text(String),
    Bytes(Vec<u8>),
}

// Define the struct for managing WebSocket connections.
pub struct ConnectionManager {
	connections: Arc<RwLock<HashMap<u64, UnboundedSender<BroadcastMessage>>>>,
	counter: AtomicU64,
}

impl ConnectionManager {
    // Create a new ConnectionManager.
    pub fn new() -> Self {
        ConnectionManager {
            connections: Arc::new(RwLock::new(HashMap::new())),
			counter: AtomicU64::new(0),
        }
    }

    // Add a new WebSocket connection to the manager.
    pub fn add_connection(&self, tx: UnboundedSender<BroadcastMessage>) -> u64 {
		let id = self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

		let mut connections = self.connections.write().unwrap();
        connections.insert(id, tx);

		id
    }

    // Remove a WebSocket connection from the manager.
    pub fn remove_connection(&self, id: u64) {
        let mut connections = self.connections.write().unwrap();
        connections.remove(&id);
    }

    // Broadcast a message to all WebSocket connections, except for the sender if given.
    pub fn broadcast(&self, message: &BroadcastMessage, sender: Option<u64>) {
        let connections = self.connections.read().unwrap();

        for (id, tx) in connections.iter() {
            if let Some(sender) = sender {
                if sender == *id {
                    continue;
                }
            }

            let _ = tx.send(message.clone());
        }
    }
}

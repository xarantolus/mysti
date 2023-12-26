use common::{ActionMessage, ClipboardContent};
use log::info;

use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::UnboundedSender;

// Define the struct for managing WebSocket connections.
pub struct Manager {
    connections: Arc<RwLock<HashMap<usize, UnboundedSender<ActionMessage>>>>,
    counter: AtomicUsize,

    pub last_clipboard_content: RwLock<ClipboardContent>,
}

impl Manager {
    // Create a new ConnectionManager.
    pub fn new() -> Self {
        Manager {
            connections: Arc::new(RwLock::new(HashMap::new())),
            counter: AtomicUsize::new(0),
            last_clipboard_content: RwLock::new(ClipboardContent::None),
        }
    }

    // Add a new WebSocket connection to the manager.
    pub fn add_connection(&self, tx: &UnboundedSender<ActionMessage>) -> usize {
        let id = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let mut connections = self.connections.write().unwrap();
        connections.insert(id, tx.clone());

        id
    }

    // Remove a WebSocket connection from the manager.
    pub fn remove_connection(&self, id: usize) {
        let mut connections = self.connections.write().unwrap();
        connections.remove(&id);
    }

    // Broadcast a message to all WebSocket connections, except for the sender if given.
    pub fn broadcast(&self, message: &ActionMessage, sender: Option<usize>) {
        let connections = self.connections.read().unwrap();

        info!(
            "Broadcasting {}message: {:?}",
            if sender.is_some() {
                sender.unwrap().to_string() + " "
            } else {
                "".to_string()
            },
            message
        );

        for (_, tx) in connections.iter().filter(|(id, _)| {
            if let Some(sender_id) = sender {
                return **id != sender_id;
            }
            true
        }) {
            let _ = tx.send(message.clone());
        }
    }
}

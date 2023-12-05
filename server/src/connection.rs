use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::UnboundedSender;

#[derive(Clone, Serialize, Deserialize)]
pub enum BroadcastMessage {
    Text(String),
    Bytes(Vec<u8>),
}

impl From<BroadcastMessage> for warp::ws::Message {
    fn from(message: BroadcastMessage) -> Self {
        match message {
            BroadcastMessage::Text(text) => warp::ws::Message::text(text),
            BroadcastMessage::Bytes(bytes) => warp::ws::Message::binary(bytes),
        }
    }
}

impl TryFrom<warp::ws::Message> for BroadcastMessage {
    type Error = anyhow::Error;

    fn try_from(message: warp::ws::Message) -> Result<Self, Self::Error> {
        if message.is_text() {
            match message.to_str() {
                Ok(msg) => Ok(BroadcastMessage::Text(msg.to_owned())),
                Err(_) => Err(anyhow::anyhow!("Error converting text message to string")),
            }
        } else if message.is_binary() {
            Ok(BroadcastMessage::Bytes(message.as_bytes().to_owned()))
        } else {
            Err(anyhow::anyhow!("Invalid message type"))
        }
    }
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
        let id = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

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

use anyhow::Result;
use common::ClipboardContent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::UnboundedSender;

#[derive(Clone, Serialize, Deserialize)]
pub enum MessageType {
    Text(String),
    Bytes(Vec<u8>),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BroadcastMessage {
    message_type: MessageType,
    sender_id: u64,
}

impl BroadcastMessage {
    pub fn from_message(message: warp::ws::Message, sender_id: u64) -> Result<Self> {
        if message.is_text() {
            match message.to_str() {
                Ok(msg) => Ok(BroadcastMessage {
                    message_type: MessageType::Text(msg.to_owned()),
                    sender_id,
                }),
                Err(_) => Err(anyhow::anyhow!("Error converting text message to string")),
            }
        } else if message.is_binary() {
            Ok(BroadcastMessage {
                message_type: MessageType::Bytes(message.as_bytes().to_owned()),
                sender_id,
            })
        } else {
            Err(anyhow::anyhow!("Invalid message type"))
        }
    }
}

impl From<BroadcastMessage> for warp::ws::Message {
    fn from(broadcast_message: BroadcastMessage) -> Self {
        match broadcast_message.message_type {
            MessageType::Text(text) => warp::ws::Message::text(text),
            MessageType::Bytes(bytes) => warp::ws::Message::binary(bytes),
        }
    }
}

// Define the struct for managing WebSocket connections.
pub struct Manager {
    connections: Arc<RwLock<HashMap<u64, UnboundedSender<BroadcastMessage>>>>,
    counter: AtomicU64,


    last_clipboard_content: ClipboardContent,
}

impl Manager {
    // Create a new ConnectionManager.
    pub fn new() -> Self {
        Manager {
            connections: Arc::new(RwLock::new(HashMap::new())),
            counter: AtomicU64::new(0),
            last_clipboard_content: ClipboardContent::None,
        }
    }

    // Add a new WebSocket connection to the manager.
    pub fn add_connection(&self, tx: &UnboundedSender<BroadcastMessage>) -> u64 {
        let id = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let mut connections = self.connections.write().unwrap();
        connections.insert(id, tx.clone());

        id
    }

    // Remove a WebSocket connection from the manager.
    pub fn remove_connection(&self, id: u64) {
        let mut connections = self.connections.write().unwrap();
        connections.remove(&id);
    }

    // Broadcast a message to all WebSocket connections, except for the sender if given.
    pub fn broadcast(&self, message: &BroadcastMessage) {
        let connections = self.connections.read().unwrap();

        for (_, tx) in connections
            .iter()
            .filter(|(&id, _)| id != message.sender_id)
        {
            let _ = tx.send(message.clone());
        }
    }
}

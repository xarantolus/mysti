use common::types::ConnectedClientInfo;
use common::{ActionMessage, ClipboardContent};
use log::{debug, error, info};

use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, RwLock};
use std::thread;
use tokio::sync::mpsc::UnboundedSender;

pub struct ConnectionInfo {
    name: String,
    pub connected_at: std::time::SystemTime,
    channel: UnboundedSender<ActionMessage>,
    supported_actions: Vec<(String, usize)>,
}

pub struct Manager {
    connections: Arc<RwLock<HashMap<usize, ConnectionInfo>>>,
    counter: AtomicUsize,
    pub(crate) config: crate::config::Config,

    pub last_clipboard_content: RwLock<ClipboardContent>,

    // In case we got a message while nobody was connected, we save it here - unless it's clipboard related
    last_message: RwLock<Option<ActionMessage>>,
}

impl Manager {
    // Create a new ConnectionManager.
    pub fn new(config: crate::config::Config) -> Self {
        Manager {
            config: config,
            connections: Arc::new(RwLock::new(HashMap::new())),
            counter: AtomicUsize::new(0),
            last_clipboard_content: RwLock::new(ClipboardContent::Text("".to_string())),
            last_message: RwLock::new(None),
        }
    }

    // Add a new WebSocket connection to the manager.
    pub fn add_connection(
        &self,
        tx: &UnboundedSender<ActionMessage>,
        name: &String,
        supported_actions: Vec<(String, usize)>,
    ) -> usize {
        let id = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let mut connections = self.connections.write().unwrap();
        connections.insert(
            id,
            ConnectionInfo {
                connected_at: std::time::SystemTime::now(),
                name: name.clone(),
                channel: tx.clone(),
                supported_actions,
            },
        );

        if let Some(last_message) = self.last_message.read().unwrap().clone() {
            tx.send(last_message).unwrap();
        }

        id
    }

    pub fn client_count(&self) -> usize {
        self.connections.read().unwrap().len()
    }

    pub fn list_clients(&self) -> Vec<ConnectedClientInfo> {
        let connections = self.connections.read().unwrap();

        connections
            .iter()
            .map(|(&id, info)| ConnectedClientInfo {
                name: info.name.clone(),
                id,
                connected_at: info.connected_at,
                supported_actions: info.supported_actions.clone(),
            })
            .collect()
    }

    // Remove a WebSocket connection from the manager.
    pub fn remove_connection(&self, id: usize) {
        let mut connections = self.connections.write().unwrap();
        connections.remove(&id);
    }

    pub fn send_to_specific(&self, id: usize, message: &ActionMessage) {
        let connections = self.connections.read().unwrap();

        if let Some(tx) = connections.get(&id) {
            let _ = tx.channel.send(message.clone());
        }
    }

    fn clipboard_action(&mut self, text: &str) {
        for action in self.config.clipboard_actions.iter() {
            let (matches, args) = action.matches(text);

            if matches {
                info!("Clipboard content matches regex: {}", action.regex);

                // Run this in a separate thread
                let action = action.clone();
                let args = args.clone();

                // Spawn thread in background, but don't wait for it to finish
                thread::spawn(move || {
                    if let Err(e) = action.run(args) {
                        error!("Error running action: {}", e);
                    }
                });
            }
        }
    }

    fn custom_message_action(&mut self, message: &ActionMessage) {
        // Sometimes we have custom logic for certain messages.
        match &message {
            ActionMessage::Clipboard(content) => {
                {
                    let mut last_clipboard_content = self.last_clipboard_content.write().unwrap();

                    // if equal content, stop
                    if *last_clipboard_content == content.clone() {
                        return;
                    }

                    *last_clipboard_content = content.clone();
                }

                debug!("Received clipboard content");

                // If the clipboard content is text, then we should run the clipboard actions.
                if let ClipboardContent::Text(text) = content {
                    self.clipboard_action(text);
                }
            }
            _ => (),
        }
    }

    // Broadcast a message to all WebSocket connections, except for the sender if given.
    pub fn broadcast(&mut self, message: &ActionMessage, sender: Option<usize>) {
        self.custom_message_action(message);

        let connections = self.connections.read().unwrap();

        if connections.is_empty() {
            // Save the message for later
            match message {
                ActionMessage::Clipboard(_) => (),
                _ => {
                    let mut last_message = self.last_message.write().unwrap();
                    *last_message = Some(message.clone());
                }
            }

            return;
        }

        info!(
            "Broadcasting message{} to {} other clients: {:?}",
            if sender.is_some() {
                " by client ".to_string() + &sender.unwrap().to_string()
            } else {
                "".to_string()
            },
            connections.len().max(1) - if sender.is_some() { 1 } else { 0 },
            message,
        );

        for (_, tx) in connections.iter().filter(|(&id, _)| {
            if let Some(sender_id) = sender {
                return id != sender_id;
            }
            true
        }) {
            let _ = tx.channel.send(message.clone());
        }
    }
}

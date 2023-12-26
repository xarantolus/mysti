use core::fmt::{self, Debug, Formatter};
use serde::{Deserialize, Serialize};

pub mod action;
pub mod client_config;
pub mod name;
pub mod url;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionMessage {
    Clipboard(ClipboardContent),
    Action(action::Action),
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClipboardContent {
    Text(String),
    Image(Vec<u8>),
    None,
}

impl Debug for ClipboardContent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ClipboardContent::Text(text) => write!(f, "Text({:?})", text),
            ClipboardContent::Image(content) => write!(f, "Image(len={})", content.len()),
            ClipboardContent::None => write!(f, "None"),
        }
    }
}

const BINARY_IMAGE_MESSAGE_TYPE: u8 = 3;

use warp::ws::Message as WebSocketMessage;

// Implement conversion from WebSocketMessage to Message and back using serde_json.
impl TryFrom<&WebSocketMessage> for ActionMessage {
    type Error = anyhow::Error;

    fn try_from(message: &WebSocketMessage) -> Result<Self, Self::Error> {
        if message.is_text() {
            match message.to_str() {
                Ok(msg) => Ok(serde_json::from_str(msg)?),
                Err(_) => Err(anyhow::anyhow!("Error converting text message to string")),
            }
        } else if message.is_binary() {
            let bytes = message.as_bytes();
            // The first byte of the binary message is the type of the message.
            if bytes.len() <= 0 {
                return Err(anyhow::anyhow!("Invalid binary message - message is empty"));
            }

            match bytes[0] {
                BINARY_IMAGE_MESSAGE_TYPE => {
                    // Image message
                    Ok(ActionMessage::Clipboard(ClipboardContent::Image(
                        bytes[1..].to_vec(),
                    )))
                }
                _ => Err(anyhow::anyhow!(
                    "Invalid binary message - invalid message type {}",
                    bytes[0]
                )),
            }
        } else {
            Err(anyhow::anyhow!("Invalid message type"))
        }
    }
}

// Implement conversion from Message to WebSocketMessage and back using serde_json.
impl TryFrom<ActionMessage> for WebSocketMessage {
    type Error = anyhow::Error;

    fn try_from(message: ActionMessage) -> Result<Self, Self::Error> {
        match message {
            // Special messages get a custom handler, otherwise just serialize the message as JSON.
            ActionMessage::Clipboard(ClipboardContent::Image(content)) => {
                let mut bytes = vec![BINARY_IMAGE_MESSAGE_TYPE];
                bytes.extend(content);
                Ok(WebSocketMessage::binary(bytes))
            }
            _ => Ok(WebSocketMessage::text(serde_json::to_string(&message)?)),
        }
    }
}

use tokio_tungstenite::tungstenite::Message;

impl TryFrom<Message> for ActionMessage {
    type Error = anyhow::Error;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        match message {
            Message::Text(msg) => Ok(serde_json::from_str(&msg)?),
            Message::Binary(bytes) => {
                // The first byte of the binary message is the type of the message.
                if bytes.len() <= 0 {
                    return Err(anyhow::anyhow!("Invalid binary message - message is empty"));
                }

                match bytes[0] {
                    BINARY_IMAGE_MESSAGE_TYPE => {
                        // Image message
                        Ok(ActionMessage::Clipboard(ClipboardContent::Image(
                            bytes[1..].to_vec(),
                        )))
                    }
                    _ => Err(anyhow::anyhow!(
                        "Invalid binary message - invalid message type {}",
                        bytes[0]
                    )),
                }
            }
            _ => Err(anyhow::anyhow!("Invalid message type")),
        }
    }
}

impl TryFrom<ActionMessage> for Message {
    type Error = anyhow::Error;

    fn try_from(message: ActionMessage) -> Result<Self, Self::Error> {
        match message {
            // Special messages get a custom handler, otherwise just serialize the message as JSON.
            ActionMessage::Clipboard(ClipboardContent::Image(content)) => {
                let mut bytes = vec![BINARY_IMAGE_MESSAGE_TYPE];
                bytes.extend(content);
                Ok(Message::Binary(bytes))
            }
            _ => Ok(Message::Text(serde_json::to_string(&message)?)),
        }
    }
}

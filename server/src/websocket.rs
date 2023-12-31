use crate::connection::Manager;
use anyhow::Result;
use common::{ActionMessage, ClipboardContent};
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info};
use std::sync::Arc;
use tokio::sync::mpsc;
use warp::{
    reply::Reply,
    ws::{Message, WebSocket},
};

#[derive(Debug, serde::Deserialize)]
pub(crate) struct DeviceInfoFilter {
    device_name: String,
    supported_actions: String,
}

pub(crate) async fn handle_client_message(
    message: ActionMessage,
    manager: Arc<Manager>,
    sender_id: Option<usize>,
) -> Result<()> {
    // Sometimes we have custom logic for certain messages.
    match &message {
        ActionMessage::Clipboard(content) => {
            let mut last_clipboard_content = manager.last_clipboard_content.write().unwrap();

            // if equal content, stop
            if *last_clipboard_content == content.clone() {
                return Ok(());
            }

            *last_clipboard_content = content.clone();

            debug!("Received clipboard content");
        }
        _ => (),
    }

    manager.broadcast(&message, sender_id);

    Ok(())
}

pub(crate) async fn handle_connection(
    ws: WebSocket,
    manager: Arc<Manager>,
    device_name: String,
    supported_actions: Vec<(String, usize)>,
) {
    let (mut user_ws_tx, mut user_ws_rx) = ws.split();
    let (websocket_writer, mut websocket_outbound_stream) = mpsc::unbounded_channel();

    let id = manager.add_connection(&websocket_writer, &device_name, supported_actions);

    // Every time we get a message from the outbound stream, send it to the user.
    tokio::spawn(async move {
        while let Some(action_msg) = websocket_outbound_stream.recv().await {
            let Ok(message) = Message::try_from(action_msg) else {
                error!("Error converting Action Message to WebSocket message");
                continue;
            };

            match user_ws_tx.send(message).await {
                Ok(_) => (),
                Err(e) => {
                    error!("Error sending message to WebSocket: {}", e);
                    break;
                }
            }
        }
    });

    // Initial message writing
    let ws_writer_clone = websocket_writer.clone();
    let manager_clone = manager.clone();
    tokio::spawn(async move {
        // Send the last clipboard content to the user
        let last_clipboard_content = manager_clone.last_clipboard_content.read().unwrap();
        let content = last_clipboard_content.clone();

        match content {
            ClipboardContent::None => (),
            _ => {
                let message = ActionMessage::Clipboard(last_clipboard_content.clone());
                let _ = ws_writer_clone.send(message);
            }
        }
    });

    log::info!(
        "Connected WebSocket connection {} ({}), now have {} connections",
        id,
        device_name,
        manager.client_count()
    );

    // Every time we get a message from the user, handle it with the handler.
    while let Some(result) = user_ws_rx.next().await {
        match result {
            Ok(message) => {
                let Ok(message) = ActionMessage::try_from(&message) else {
                    if message.is_ping() || message.is_close() {
                        continue;
                    }

                    error!(
                        "Error converting WebSocket message {:?} to Action Message",
                        message
                    );
                    continue;
                };

                if let Err(e) = handle_client_message(message, manager.clone(), Some(id)).await {
                    error!("Error handling message from WebSocket: {}", e);
                }
            }
            Err(e) => {
                error!("Error receiving message from WebSocket: {}", e);
                break;
            }
        }
    }

    manager.remove_connection(id);

    info!(
        "WebSocket connection closed for {}, now have {} clients",
        id,
        manager.client_count()
    );
}

pub(crate) fn handle_ws_route(
    device_info: DeviceInfoFilter,
    ws: warp::ws::Ws,
    manager: Arc<Manager>,
) -> impl Reply {
    ws.on_upgrade(move |socket| {
        handle_connection(
            socket,
            manager,
            device_info.device_name,
            device_info
                .supported_actions
                .split(",")
                .filter_map(|pair| {
                    let (key, value_str) = pair.split_once(":")?;
                    let key = key.trim().to_string();
                    let value = value_str.trim().parse().ok()?;
                    Some((key, value))
                })
                .collect(),
        )
    })
}

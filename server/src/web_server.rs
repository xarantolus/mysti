use crate::config::Config;
use crate::Manager;
use anyhow::Result;
use common::{ActionMessage, ClipboardContent};
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info};
use std::net::SocketAddr;
use warp::reply::Reply;

use std::{convert::Infallible, sync::Arc};
use tokio::sync::mpsc;
use warp::ws::{Message, WebSocket};
use warp::Filter;
use wol::send_wol;

fn with_manager(
    manager: Arc<Manager>,
) -> impl Filter<Extract = (Arc<Manager>,), Error = Infallible> + Clone {
    warp::any().map(move || manager.clone())
}

fn with_config(
    config: Arc<Config>,
) -> impl Filter<Extract = (Arc<Config>,), Error = Infallible> + Clone {
    warp::any().map(move || config.clone())
}

async fn handle_client_message(
    message: ActionMessage,
    manager: Arc<Manager>,
    sender_id: u64,
) -> Result<()> {
    manager.broadcast(&message, Some(sender_id));

    // Sometimes we have custom logic for certain messages.
    match message {
        ActionMessage::Clipboard(content) => {
            let mut last_clipboard_content = manager.last_clipboard_content.write().unwrap();
            *last_clipboard_content = content;

            debug!("Received clipboard content");
        }
        _ => (),
    }

    Ok(())
}

async fn handle_connection(ws: WebSocket, manager: Arc<Manager>) {
    let (mut user_ws_tx, mut user_ws_rx) = ws.split();
    let (websocket_writer, mut websocket_outbound_stream) = mpsc::unbounded_channel();

    let id = manager.add_connection(&websocket_writer);

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

    // Every time we get a message from the user, handle it with the handler.
    while let Some(result) = user_ws_rx.next().await {
        match result {
            Ok(message) => {
                let Ok(message) = ActionMessage::try_from(message) else {
                    error!("Error converting WebSocket message to Message");
                    continue;
                };

                if let Err(e) = handle_client_message(message, manager.clone(), id).await {
                    error!("Error handling message from WebSocket: {}", e);
                }
            }
            Err(e) => {
                error!("Error receiving message from WebSocket: {}", e);
                break;
            }
        }
    }

    info!("WebSocket connection closed for {}", id);
    manager.remove_connection(id);
}

pub async fn start_web_server(config: &Config, connection_manager: Arc<Manager>) {
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(with_manager(connection_manager.clone()))
        .map(|ws: warp::ws::Ws, manager: Arc<Manager>| {
            ws.on_upgrade(move |socket| handle_connection(socket, manager))
        });

    let broadcast_route = warp::path("broadcast")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_manager(connection_manager.clone()))
        .map(|message: ActionMessage, manager: Arc<Manager>| {
            manager.broadcast(&message, None);
            warp::reply()
        });

    let wake_on_lan_route = warp::path("wol")
        .and(warp::post())
        .and(with_config(Arc::new(config.clone())))
        .map(|config: Arc<Config>| {
            let res = send_wol(
                config.wake_on_lan.target_addr,
                config.wake_on_lan.router_addr,
                None,
            );

            log::info!("Sending WoL packet to {}", config.wake_on_lan.target_addr);

            match res {
                Ok(()) => warp::reply::with_status(
                    warp::reply::html("Starting PC"),
                    warp::http::StatusCode::OK,
                )
                .into_response(),
                Err(e) => warp::reply::with_status(
                    warp::reply::json(&e.to_string()),
                    warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                )
                .into_response(),
            }
        });

    let routes = ws_route.or(broadcast_route).or(wake_on_lan_route);

    let addr: SocketAddr = ("[::]:".to_owned() + &config.web_port.to_string())
        .parse()
        .unwrap();

    info!("Starting web server on port {}", config.web_port);
    warp::serve(routes).run(addr).await;
}

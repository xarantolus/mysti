use crate::config::Config;
use crate::Manager;
use anyhow::Result;
use common::action::ActionWrapper;
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

    manager.broadcast(&message, Some(sender_id));

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

fn handle_ws_route(ws: warp::ws::Ws, manager: Arc<Manager>) -> impl Reply {
    ws.on_upgrade(move |socket| handle_connection(socket, manager))
}

fn handle_wake_on_lan_route(config: Arc<Config>) -> impl Reply {
    let res = send_wol(
        config.wake_on_lan.target_addr,
        config.wake_on_lan.router_addr,
        None,
    );

    log::info!("Sending WoL packet to {}", config.wake_on_lan.target_addr);

    match res {
        Ok(()) => {
            warp::reply::with_status(warp::reply::html("Starting PC"), warp::http::StatusCode::OK)
                .into_response()
        }
        Err(e) => warp::reply::with_status(
            warp::reply::json(&e.to_string()),
            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
        )
        .into_response(),
    }
}

/// get a JSON message like {"action": "shutdown"} and broadcast it as an ActionMessage::Action
fn handle_action_route(wrapper: ActionWrapper, manager: Arc<Manager>) -> impl Reply {
    manager.broadcast(&ActionMessage::Action(wrapper.action), None);
    warp::reply::html("OK")
}

pub async fn start_web_server(config: &Config, connection_manager: Arc<Manager>) {
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(with_manager(connection_manager.clone()))
        .map(handle_ws_route);

    let wake_on_lan_route = warp::path("wol")
        .and(warp::post())
        .and(with_config(Arc::new(config.clone())))
        .map(handle_wake_on_lan_route);

    let action_route = warp::path!("actions" / "create")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_manager(connection_manager.clone()))
        .map(handle_action_route);

    let routes = ws_route.or(action_route).or(wake_on_lan_route);

    let addr: SocketAddr = ("[::]:".to_owned() + &config.web_port.to_string())
        .parse()
        .unwrap();

    info!("Starting web server on port {}", config.web_port);
    warp::serve(routes).run(addr).await;
}

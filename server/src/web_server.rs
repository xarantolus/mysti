use crate::config::Config;
use crate::Manager;
use anyhow::Result;
use common::action::Action;
use common::{ActionMessage, ClipboardContent};
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info};
use std::net::SocketAddr;
use warp::reject::Rejection;
use warp::reply::Reply;

use std::{convert::Infallible, sync::Arc};
use tokio::sync::mpsc;
use wake_on_lan::MagicPacket;
use warp::ws::{Message, WebSocket};
use warp::Filter;

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

#[derive(Debug, serde::Deserialize)]
struct DeviceInfoFilter {
    device_name: String,
    supported_actions: String,
}

async fn handle_client_message(
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

async fn handle_connection(
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

fn handle_ws_route(
    _: bool,
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

fn handle_wake_on_lan_route(_: bool, config: Arc<Config>) -> impl Reply {
    let magic_packet = MagicPacket::new(&config.wake_on_lan.target_addr.0.into_array());

    let res = magic_packet.send();

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
fn handle_action_route(_: bool, wrapper: Action, manager: Arc<Manager>) -> impl Reply {
    manager.broadcast(&ActionMessage::Action(wrapper), None);
    warp::reply::html("OK")
}

fn handle_specific_action_route(
    id: usize,
    _: bool,
    wrapper: Action,
    manager: Arc<Manager>,
) -> impl Reply {
    manager.send_to_specific(id, &ActionMessage::Action(wrapper));
    warp::reply::html("OK")
}

fn handle_read_clipboard_route(_: bool, manager: Arc<Manager>) -> impl Reply {
    let last_clipboard_content = manager.last_clipboard_content.read().unwrap();

    let text = match last_clipboard_content.clone() {
        ClipboardContent::Text(text) => text,
        _ => "No clipboard content".to_string(),
    };

    warp::reply::html(text)
}

fn handle_write_clipboard_route(
    _: bool,
    body: warp::hyper::body::Bytes,
    manager: Arc<Manager>,
) -> impl Reply {
    let text = String::from_utf8_lossy(&body).to_string();

    let result = futures::executor::block_on(handle_client_message(
        ActionMessage::Clipboard(ClipboardContent::Text(text.clone())),
        manager,
        None,
    ));

    match result {
        Ok(_) => warp::reply::html(text).into_response(),
        Err(e) => warp::reply::with_status(
            warp::reply::json(&e.to_string()),
            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
        )
        .into_response(),
    }
}

fn handle_client_list(_: bool, manager: Arc<Manager>) -> impl Reply {
    warp::reply::json(&manager.list_clients())
}

// Define a struct to represent the query parameters
#[derive(serde::Deserialize)]
struct AuthQuery {
    token: String,
}

// Define a filter for authentication
fn with_auth(token: String) -> impl Filter<Extract = (bool,), Error = Rejection> + Clone {
    warp::any()
        .and(warp::filters::query::query::<AuthQuery>())
        .map(move |query: AuthQuery| query.token == token)
}

pub async fn start_web_server(config: &Config, connection_manager: Arc<Manager>) {
    let ws_route = warp::path("ws")
        .and(with_auth(config.token.to_string()))
        .and(warp::query::<DeviceInfoFilter>())
        .and(warp::ws())
        .and(with_manager(connection_manager.clone()))
        .map(handle_ws_route);

    let wake_on_lan_route = warp::path("wol")
        .and(with_auth(config.token.to_string()))
        .and(warp::post())
        .and(with_config(Arc::new(config.clone())))
        .map(handle_wake_on_lan_route);

    let action_route = warp::path!("actions" / "create")
        .and(with_auth(config.token.to_string()))
        .and(warp::post())
        .and(warp::body::json())
        .and(with_manager(connection_manager.clone()))
        .map(handle_action_route);

    let action_route_specific = warp::path!("actions" / "create" / usize)
        .and(with_auth(config.token.to_string()))
        .and(warp::post())
        .and(warp::body::json())
        .and(with_manager(connection_manager.clone()))
        .map(handle_specific_action_route);

    let clipboard_read_route = warp::path!("devices" / "clipboard")
        .and(with_auth(config.token.to_string()))
        .and(warp::get())
        .and(with_manager(connection_manager.clone()))
        .map(handle_read_clipboard_route);

    let clipboard_write_route = warp::path!("devices" / "clipboard")
        .and(with_auth(config.token.to_string()))
        .and(warp::post())
        .and(warp::body::content_length_limit(1024 * 32))
        .and(warp::body::bytes())
        .and(with_manager(connection_manager.clone()))
        .map(handle_write_clipboard_route);

    let client_list_route = warp::path!("devices")
        .and(with_auth(config.token.to_string()))
        .and(warp::get())
        .and(with_manager(connection_manager.clone()))
        .map(handle_client_list);

    let routes = ws_route
        .or(action_route)
        .or(action_route_specific)
        .or(wake_on_lan_route)
        .or(client_list_route)
        .or(clipboard_read_route)
        .or(clipboard_write_route);

    let addr: SocketAddr = ("[::]:".to_owned() + &config.web_port.to_string())
        .parse()
        .unwrap();

    info!("Starting web server on port {}", config.web_port);
    warp::serve(routes).run(addr).await;
}

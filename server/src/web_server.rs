use crate::Manager;
use anyhow::Result;
use common::ActionMessage;
use futures_util::{SinkExt, StreamExt};
use std::{convert::Infallible, sync::Arc};
use tokio::sync::mpsc;
use warp::ws::{Message, WebSocket};
use warp::Filter;

fn with_manager(
    manager: Arc<Manager>,
) -> impl Filter<Extract = (Arc<Manager>,), Error = Infallible> + Clone {
    warp::any().map(move || manager.clone())
}

async fn handle_client_message(
    message: ActionMessage,
    manager: Arc<Manager>,
    sender_id: u64,
) -> Result<()> {
    // TODO: some custom logic to copy clipboard content to manager struct
    manager.broadcast(&message, Some(sender_id));

    // Sometimes we have custom logic for certain messages.
    match message {
        ActionMessage::Clipboard(content) => {
            let mut last_clipboard_content = manager.last_clipboard_content.write().unwrap();
            *last_clipboard_content = content;
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
                eprintln!("Error converting Action Message to WebSocket message");
                continue;
            };

            match user_ws_tx.send(message).await {
                Ok(_) => (),
                Err(e) => {
                    eprintln!("Error sending message to WebSocket: {}", e);
                    break;
                }
            }
        }
    });

    // Every time we get a message from the user, handle it with the handler.
    while let Some(result) = user_ws_rx.next().await {
        match result {
            Ok(message) => {
                let Ok(message) = ActionMessage::try_from(message) else {
                    eprintln!("Error converting WebSocket message to Message");
                    continue;
                };

                if let Err(e) = handle_client_message(message, manager.clone(), id).await {
                    eprintln!("Error handling message from WebSocket: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Error receiving message from WebSocket: {}", e);
                break;
            }
        }
    }

    eprintln!("WebSocket connection closed for {}", id);
    manager.remove_connection(id);
}

pub async fn start_web_server(web_port: u16, connection_manager: Arc<Manager>) {
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

    let routes = ws_route.or(broadcast_route);

    println!("Starting web server on port {}", web_port);
    warp::serve(routes).run(([127, 0, 0, 1], web_port)).await;
}

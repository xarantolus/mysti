use crate::config::Config;
use crate::websocket::{handle_client_message, handle_ws_route, DeviceInfoFilter};
use crate::Manager;
use common::action::Action;
use common::{ActionMessage, ClipboardContent};
use log::info;
use std::net::SocketAddr;
use warp::reject::Rejection;
use warp::reply::Reply;
use subtle::ConstantTimeEq;

use std::{convert::Infallible, sync::Arc};

use wake_on_lan::MagicPacket;
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

fn handle_wake_on_lan_route(config: Arc<Config>) -> impl Reply {
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
fn handle_action_route(wrapper: Action, manager: Arc<Manager>) -> impl Reply {
    manager.broadcast(&ActionMessage::Action(wrapper), None);
    warp::reply::html("OK")
}

fn handle_specific_action_route(
    id: usize,
    wrapper: Action,
    manager: Arc<Manager>,
) -> impl Reply {
    manager.send_to_specific(id, &ActionMessage::Action(wrapper));
    warp::reply::html("OK")
}

fn handle_read_clipboard_route(manager: Arc<Manager>) -> impl Reply {
    let last_clipboard_content = manager.last_clipboard_content.read().unwrap();

    let text = match last_clipboard_content.clone() {
        ClipboardContent::Text(text) => text,
        _ => "No clipboard content".to_string(),
    };

    warp::reply::html(text)
}

fn handle_write_clipboard_route(
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

fn handle_client_list(manager: Arc<Manager>) -> impl Reply {
    warp::reply::json(&manager.list_clients())
}

// Define a struct to represent the query parameters
#[derive(serde::Deserialize)]
struct AuthQuery {
    token: String,
}

// Define a filter for authentication
fn with_auth(token: String) -> impl Filter<Extract = (), Error = Rejection> + Clone {
    warp::any()
        .and(warp::filters::query::query::<AuthQuery>())
        .map(move |query: AuthQuery| query.token.as_bytes().ct_eq(token.as_bytes()).into())
        .and_then(|is_valid| async move {
            if is_valid {
                Ok(())
            } else {
                Err(warp::reject::not_found())
            }
        })
        .untuple_one()
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

use std::sync::Arc;

mod web_server;
use connection::Manager;
use log::info;
use web_server::start_web_server;

mod connection;
mod server_action;
mod websocket;

mod config;
use config::parse_file;

#[tokio::main]
async fn main() {
    env_logger::init();

    let config = parse_file("config.toml").expect("Failed to parse config file");

    info!(
        "Loaded config with {} clipboard actions",
        config.clipboard_actions.len()
    );

    let server_data = Arc::new(Manager::new(config.clone()));

    start_web_server(&config, server_data).await;
}

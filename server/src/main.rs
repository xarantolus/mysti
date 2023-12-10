use std::sync::Arc;

mod web_server;
use connection::Manager;
use web_server::start_web_server;

mod connection;

mod config;
use config::parse_file;

#[tokio::main]
async fn main() {
    env_logger::init();

    let config = parse_file("config.toml").expect("Failed to parse config file");

    let server_data = Arc::new(Manager::new());

    start_web_server(&config, server_data).await;
}

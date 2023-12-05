use anyhow::Context;
use std::sync::{
    Arc, Mutex,
};


mod web_server;
use connection::ConnectionManager;
use web_server::start_web_server;

mod connection;


#[actix_web::main]
async fn main() {
    let web_port = 8080;

    let server_data = Arc::new(Mutex::new(ConnectionManager::new()));

    start_web_server(web_port, server_data).expect("Failed to start web server").await.expect("Failed to await start of web server")
}

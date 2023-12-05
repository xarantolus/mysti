use std::sync::Arc;

mod web_server;
use connection::Manager;
use web_server::start_web_server;

mod connection;

#[actix_web::main]
async fn main() {
    let web_port = 8080;

    let server_data = Arc::new(Manager::new());

    start_web_server(web_port, server_data).await;
}

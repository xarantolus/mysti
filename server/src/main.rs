use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc, Mutex,
};

use actix::{Actor, Context};
use actix_web::{
    web::{Bytes},
};


mod web_server;
use connection::ConnectionManager;
use web_server::start_web_server;

mod connection;


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let web_port = 8080;

    let server_data = Arc::new(Mutex::new(ConnectionManager::new()));

    // Run web server in separate thread
    actix::spawn(async move {
        start_web_server(web_port, server_data).expect("Failed to start web server").await.expect("Failed to await start of web server")
    });

    // start_socket_server().await

    loop   {
        println!("Hello world!");

        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

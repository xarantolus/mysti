use actix_web::{web, App, HttpServer};
use anyhow::{Context, Result};
use std::sync::{Arc, Mutex};

use crate::ConnectionManager;

pub fn start_web_server(
    web_port: u16,
    connection_manager: Arc<Mutex<ConnectionManager>>,
) -> Result<actix_web::dev::Server> {
    println!("Starting web server on port {}", web_port);

    Ok(HttpServer::new(move || {
        let wd = web::Data::new(connection_manager.clone());

        App::new()
            .app_data(wd)
            .route("/test", web::get().to(|| async { "Hello world!" }))
    })
    .bind(("127.0.0.1", web_port))
    .context("Failed to bind web server to port")?
    .run())
}

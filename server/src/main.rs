use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc, Mutex,
};

use actix::{Actor, StreamHandler, Message, Handler, Addr, Context};
use actix_web::{
    web::{self, Bytes},
    App, Error, HttpRequest, HttpResponse, HttpServer,
};
use actix_web_actors::ws::{self, WebsocketContext};

mod web_server;
use web_server::start_web_server;

#[derive(Clone)]
enum BroadcastMessage {
    Text(String),
    Bytes(Bytes),
}

#[derive(Clone)]
struct Server {
    clients: Arc<Mutex<Vec<Arc<SingleWSClient>>>>,
}

impl Actor for Server {
    type Context = Context<Self>;
}

impl Server {
    fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn broadcast(&self, msg: BroadcastMessage) {
        let clients = self.clients.lock().unwrap();
        for client in clients.iter() {
            // client.
        }
    }
}


#[derive(Clone)]
struct SingleWSClient {
    incoming: Sender<BroadcastMessage>,
    outgoing: Arc<Mutex<Receiver<BroadcastMessage>>>,
}

impl SingleWSClient {
    fn new() -> Self {
        let (incoming, outgoing) = channel();
        Self {
            incoming,
            outgoing: Arc::new(Mutex::new(outgoing)),
        }
    }
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let web_port = 8080;

    let server_data = Arc::new(Mutex::new(Server::new()));

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

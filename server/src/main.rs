use actix::{Actor, StreamHandler};
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;

/// Define HTTP actor
struct SingleWSClient;

impl Actor for SingleWSClient {
    type Context = ws::WebsocketContext<Self>;
}

/// Handler for ws::Message message
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for SingleWSClient {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let Ok(message) = msg else {
            println!("WS ProtocolError: {:?}", msg);
            return;
        };

        match message {
            ws::Message::Ping(msg) => ctx.pong(&msg),
            ws::Message::Text(text) => ctx.text(text),
            ws::Message::Binary(bin) => ctx.binary(bin),
            ws::Message::Close(reason) => {
                println!("Closed WS connection: {:?}", reason);
            }
            _ => (),
        }
    }
}

async fn index(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    let client = SingleWSClient {};
    let resp = ws::start(client, &req, stream);

    // TODO: Somehow join into clients list and then broadcast messages to all clients


    resp
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = 8080;

    println!("Starting server on port {}", port);

    HttpServer::new(|| App::new().route("/ws", web::get().to(index)))
        .bind(("127.0.0.1", port))?
        .run()
        .await
}

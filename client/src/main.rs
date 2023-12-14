mod clipboard;

use crate::clipboard::Watcher;
use anyhow::{Context, Result};
use common::{ActionMessage, ClipboardContent};
use futures_util::SinkExt;
use futures_util::StreamExt;
use image::ImageOutputFormat;
use std::convert::TryInto;
use std::{thread, time::Duration};
use tokio::select;
use tokio::sync::mpsc::channel;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::connect_async;
use url::Url;

enum LocalEvent {
    ClipboardEvent(ClipboardContent),
}

enum Event {
    LocalEvent(LocalEvent),
    RemoteEvent(ActionMessage),
    OutgoingEvent(ActionMessage),
}

impl From<ClipboardContent> for LocalEvent {
    fn from(content: ClipboardContent) -> Self {
        Self::ClipboardEvent(content)
    }
}

struct MystiClient {
    server_url: String,
    image_format: ImageOutputFormat,
}

impl MystiClient {
    fn new(server_url: String, image_format: ImageOutputFormat) -> Self {
        Self {
            server_url,
            image_format,
        }
    }

    async fn on_local_clipboard_change(&self, content: ClipboardContent, channel: Sender<Event>) {
        let am = ActionMessage::Clipboard(content);

        channel
            .send(Event::OutgoingEvent(am))
            .await
            .expect("Failed to send clipboard content");
    }

    async fn process_local_event(&self, event: LocalEvent, channel: Sender<Event>) {
        match event {
            LocalEvent::ClipboardEvent(content) => {
                self.on_local_clipboard_change(content, channel).await;
            }
        }
    }

    async fn process_action_message(&mut self, event: &ActionMessage) -> Result<()> {
        eprintln!("Received action message: {:?}", event);

        match &event {
            ActionMessage::Clipboard(content) => {
                clipboard::set_clipboard(&content)?;
            }
            ActionMessage::Action(action) => {
                action.run().await?;
            }
        }

        Ok(())
    }

    async fn run(&mut self) -> Result<()> {
        // copy the sender, creating a new one
        let (clipboard_events, mut clipboard_receiver) = channel::<LocalEvent>(10);

        // Run in a separate thread
        let mut w = Watcher::new(self.image_format.clone(), clipboard_events.clone());
        thread::spawn(move || {
            w.run().expect("Failed to run watcher");
        });

        // Parse and set the correct URL
        let mut server_url = Url::parse(&self.server_url).context("Failed to parse server URL")?;
        server_url.set_path("/ws");
        server_url
            .set_scheme(match server_url.scheme() {
                "http" => "ws",
                "https" => "wss",
                _ => return Err(anyhow::anyhow!("Invalid scheme")),
            })
            .map_err(|_| anyhow::anyhow!("Failed to set scheme"))?;
        server_url.set_query(Some(
            "token=c8e974b3313f0d67f66eaf449b3df7p785c7bb6eaeade7e5b1cfba3a9ddc48ed6",
        ));

        let (remote_event, mut remote_receiver) = channel::<ActionMessage>(10);
        let (outgoing_events, mut outgoing_receiver) = channel::<ActionMessage>(10);

        let moved_server_url = server_url.clone();
        tokio::spawn(async move {
            loop {
                println!("Connecting to {}", moved_server_url);

                // Attempt to connect to server and retry if it fails
                let socket = loop {
                    match connect_async(moved_server_url.clone()).await {
                        Ok((socket, _)) => break socket,
                        Err(e) => {
                            eprintln!("Failed to connect to server: {}", e);
                            tokio::time::sleep(Duration::from_secs(5)).await;
                        }
                    };
                };

                println!("Connected to server");

                let (mut socket_sender, mut socket_receiver) = socket.split();

                let mut ping_interval = tokio::time::interval(Duration::from_secs(30));

                loop {
                    // Read something from the socket OR write something to the socket when we get an outgoing event
                    select! {
                        // Simple example with correct sytax: receive from outgoing_receiver and socket_receiver
                        event = outgoing_receiver.recv() => {
                            println!("Received outgoing event: {:?}", event);
                            let Some(event) = event else { break };

                            let message : tokio_tungstenite::tungstenite::Message = match event.try_into() {
                                Ok(message) => message,
                                Err(err) => {
                                    eprintln!("Failed to convert event to message: {}", err);
                                    continue;
                                }
                            };

                            if let Err(e) = socket_sender.send(message).await {
                                eprintln!("Failed to send message to server: {}", e);
                                break;
                            }
                        }
                        event = socket_receiver.next() => {
                            let Some(event) = event else { break };
                            let Ok(event) = event else {
                                eprintln!("Failed to receive remote event: {:?}", event);
                                break;
                            };

                            match event {
                                tokio_tungstenite::tungstenite::Message::Close(_) => {
                                    eprintln!("Server sent close");
                                    break;
                                }
                                tokio_tungstenite::tungstenite::Message::Pong(_) => {
                                    continue;
                                }
                                _ => (),
                            };

                            println!("Received remote event: {:?}", event);

                            let action_message : ActionMessage = match event.try_into() {
                                Ok(event) => event,
                                Err(err) => {
                                    eprintln!("Failed to convert message to event: {}", err);
                                    continue;
                                }
                            };

                            remote_event.send(action_message).await.expect("Failed to send remote event");
                        }
                        _ = ping_interval.tick() => {
                            if let Err(e) = socket_sender.send(tokio_tungstenite::tungstenite::Message::Ping(vec![])).await {
                                eprintln!("Failed to send ping: {}", e);
                                break;
                            }
                        }
                    }
                }

                println!("Disconnected from server - reconnecting in 5 seconds");
                thread::sleep(Duration::from_secs(5));
            }
        });

        let (all_events, mut all_receiver) = channel::<Event>(10);

        // Forward local events to the main channel
        let local_all_events = all_events.clone();
        tokio::spawn(async move {
            loop {
                let event = clipboard_receiver
                    .recv()
                    .await
                    .expect("Failed to receive local event");
                local_all_events
                    .send(Event::LocalEvent(event))
                    .await
                    .expect("Failed to send local event");
            }
        });

        // Forward remote events to the main channel
        let remote_all_events = all_events.clone();
        tokio::spawn(async move {
            loop {
                let event = remote_receiver
                    .recv()
                    .await
                    .expect("Failed to receive remote event");
                remote_all_events
                    .send(Event::RemoteEvent(event))
                    .await
                    .expect("Failed to send remote event");
            }
        });

        loop {
            let event = all_receiver.recv().await.expect("Failed to receive event");

            match event {
                Event::LocalEvent(event) => {
                    let event_return = all_events.clone();

                    self.process_local_event(event, event_return).await;
                }
                Event::RemoteEvent(event) => match self.process_action_message(&event).await {
                    Ok(_) => (),
                    Err(err) => {
                        eprintln!("Error processing action message {:?}: {}", event, err);
                    }
                },
                Event::OutgoingEvent(event) => {
                    outgoing_events
                        .send(event)
                        .await
                        .expect("Failed to send outgoing event");
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let mut client = MystiClient::new(
        "https://misc.010.one:728/".to_string(),
        ImageOutputFormat::Bmp,
    );

    client.run().await.expect("Failed to run client");
}

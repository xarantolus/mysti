mod clipboard;
mod config;

use crate::clipboard::Watcher;
use anyhow::{Context, Result};
use common::{ActionMessage, ClipboardContent};
use config::Config;
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
    config: Config,
    image_format: ImageOutputFormat,
}

impl MystiClient {
    fn new(config: Config, image_format: ImageOutputFormat) -> Self {
        Self {
            config,
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
        log::info!("Received action message: {:?}", event);

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
        let mut server_url =
            Url::parse(&self.config.server_host).context("Failed to parse server URL")?;
        server_url.set_path("/ws");
        server_url
            .set_scheme(match server_url.scheme() {
                "http" => "ws",
                "https" => "wss",
                "ws" => "ws",
                "wss" => "wss",
                "" => "wss",
                _ => return Err(anyhow::anyhow!("Invalid scheme")),
            })
            .map_err(|_| anyhow::anyhow!("Failed to set scheme"))?;

        server_url
            .query_pairs_mut()
            .append_pair("token", &self.config.token);

        let (remote_event, mut remote_receiver) = channel::<ActionMessage>(10);
        let (outgoing_events, mut outgoing_receiver) = channel::<ActionMessage>(10);

        let moved_server_url = server_url.clone();
        tokio::spawn(async move {
            loop {
                log::info!("Connecting to {}", moved_server_url);

                // Attempt to connect to server and retry if it fails
                let socket = {
                    let mut fail_count = 0;
                    loop {
                        match connect_async(moved_server_url.clone()).await {
                            Ok((socket, _)) => break socket,
                            Err(e) => {
                                if fail_count % 12 == 0 {
                                    log::warn!("Failed to connect to server: {}", e);
                                }
                                fail_count += 1;

                                tokio::time::sleep(Duration::from_secs(5)).await;
                            }
                        };
                    }
                };

                println!("Connected to server");

                let (mut socket_sender, mut socket_receiver) = socket.split();

                let mut ping_interval = tokio::time::interval(Duration::from_secs(60));

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
                                    log::warn!("Failed to convert event to message: {}", err);
                                    continue;
                                }
                            };

                            if let Err(e) = socket_sender.send(message).await {
                                log::warn!("Failed to send message to server: {}", e);
                                break;
                            }
                        }
                        event = socket_receiver.next() => {
                            let Some(event) = event else { break };
                            let Ok(event) = event else {
                                log::warn!("Failed to receive remote event: {:?}", event);
                                break;
                            };

                            match event {
                                tokio_tungstenite::tungstenite::Message::Close(_) => {
                                    log::warn!("Server sent close");
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
                                    log::warn!("Failed to convert message to event: {}", err);
                                    continue;
                                }
                            };

                            remote_event.send(action_message).await.expect("Failed to send remote event");
                        }
                        _ = ping_interval.tick() => {
                            if let Err(e) = tokio::time::timeout(Duration::from_secs(5), socket_sender.send(tokio_tungstenite::tungstenite::Message::Ping(vec![1,2,3,4]))).await {
                                log::warn!("Failed to send ping: {}", e);
                                break;
                            }

                            // Receive pong
                            let Ok(Some(Ok(event))) = tokio::time::timeout(Duration::from_secs(5), socket_receiver.next()).await else {
                                log::warn!("Failed to receive pong");
                                break;
                            };
                            if let tokio_tungstenite::tungstenite::Message::Pong(_) = event {
                                continue;
                            } else {
                                log::warn!("Received non-pong message");
                                break;
                            }
                        }
                    }
                }

                println!("Disconnected from server - reconnecting in 5 seconds");
                tokio::time::sleep(Duration::from_secs(5)).await;
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
                        log::warn!("Error processing action message {:?}: {}", event, err);
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
    // log to mysti-client.log (via fern) and stdout
    fern::Dispatch::new()
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(fern::log_file("mysti-client.log").expect("Failed to open log file"))
        .apply()
        .expect("Failed to initialize logger");

    let config = config::find_parse_config().expect("Failed to parse config");

    let mut client = MystiClient::new(config, ImageOutputFormat::Bmp);

    client.run().await.expect("Failed to run client");
}

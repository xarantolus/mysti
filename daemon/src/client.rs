use crate::clipboard::{self, Watcher};
use anyhow::Result;
use common::action::ActionDefinition;
use common::name::client_name;
use common::url;
use common::{client_config::ClientConfig, ActionMessage, ClipboardContent};
use futures_util::SinkExt;
use futures_util::StreamExt;
use image::ImageOutputFormat;
use std::convert::TryInto;
use std::{thread, time::Duration};
use tokio::select;
use tokio::sync::mpsc::channel;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::connect_async;

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

pub struct MystiClient {
    config: ClientConfig,
    image_format: ImageOutputFormat,
}

impl MystiClient {
    pub fn new(config: ClientConfig, image_format: ImageOutputFormat) -> Self {
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
                let action_definition =
                    ActionDefinition::find_by_name(&action.action, &self.config.actions);

                match action_definition {
                    Some(action_definition) => {
                        action_definition.run(&action.args)?;
                    }
                    None => {
                        log::warn!("Action {} not found", action.action);
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        // copy the sender, creating a new one
        let (clipboard_events, mut clipboard_receiver) = channel::<LocalEvent>(10);

        // Run in a separate thread
        let mut w = Watcher::new(self.image_format.clone(), clipboard_events.clone());
        thread::spawn(move || {
            w.run().expect("Failed to run watcher");
        });

        // Parse and set the correct URL
        let mut server_url =
            url::generate_request_url(&self.config, "/ws", url::Scheme::WebSocket)?;

        server_url
            .query_pairs_mut()
            .append_pair(
                "supported_actions",
                &self
                    .config
                    .actions
                    .iter()
                    .filter(|a| a.is_available())
                    .map(|a| format!("{}:{}", a.name, a.required_args()))
                    .collect::<Vec<String>>()
                    .join(","),
            )
            .append_pair("device_name", &client_name());

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

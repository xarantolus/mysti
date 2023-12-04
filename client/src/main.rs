mod clipboard;

use std::{sync::{Arc, Mutex}, thread, time::Duration, sync::mpsc::{channel, self}};

use crate::clipboard::{Watcher, ClipboardContent};
use anyhow::Result;
use image::ImageOutputFormat;

enum Event {
    ClipboardEvent(ClipboardContent),
}

impl From<ClipboardContent> for Event {
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


    fn on_local_clipboard_change(&self, content: ClipboardContent) {
        // TODO: Send to server
        match content {
            ClipboardContent::Text(text) => {
                println!("Clipboard text: {}", text);
            }
            ClipboardContent::Image(bytes, format) => {
                println!("Clipboard image: {} bytes, format: {:?}", bytes.len(), format);
            }
            clipboard::ClipboardContent::None => {
                println!("Clipboard empty");
            }
        }
    }

    fn process_event(&self, event: Event) {
        match event {
            Event::ClipboardEvent(content) => {
                self.on_local_clipboard_change(content);
            }
        }
    }

    fn run(&self) -> Result<()> {
        // copy the sender, creating a new one
        let (sender, receiver) = channel();

        // Run in a separate thread
        let mut w = Watcher::new(self.image_format.clone(), sender.clone());
        thread::spawn(move || {
            w.run().expect("Failed to run watcher");
        });

        // TODO: spawn some websocket connection to server that sends events as well

        loop {
            // Wait for an event
            let event = receiver.recv().expect("Failed to receive event");
            self.process_event(event);
        }
    }
}

fn main() {
    let client = MystiClient::new("http://localhost:8000".to_string(), ImageOutputFormat::Bmp);

    client.run().expect("Failed to run client");
}

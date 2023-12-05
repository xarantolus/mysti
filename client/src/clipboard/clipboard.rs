use anyhow::Context;
use anyhow::Result;
use clipboard_master::{CallbackResult, ClipboardHandler, Master};
use common::ClipboardContent;
use image::ImageOutputFormat;
use image::RgbaImage;
use std::io;
use std::io::Cursor;
use std::sync::mpsc::Sender;

use arboard::Clipboard;
use arboard::ImageData;
use image::DynamicImage;

pub struct Watcher<T: From<ClipboardContent>> {
    // A channel of objects that can be ClipboardContent.into() converted
    channel: Sender<T>,
    output_format: ImageOutputFormat,
}

impl<T: From<ClipboardContent>> Watcher<T> {
    pub fn new(output_format: ImageOutputFormat, sender: Sender<T>) -> Self {
        Self {
            channel: sender,
            output_format,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        Master::new(self)
            .run()
            .context("failed to run clipboard watcher")
    }
}

fn to_dynamic_image(image: ImageData) -> Result<DynamicImage> {
    Ok(DynamicImage::ImageRgba8(
        RgbaImage::from_raw(
            image.width as u32,
            image.height as u32,
            image.bytes.into_owned(),
        )
        .context("failed to decode image")?,
    ))
}

// Gets the actual clipboard content
fn get_clipboard_content(output_format: &ImageOutputFormat) -> Result<ClipboardContent> {
    let mut clipboard = Clipboard::new()?;
    if let Ok(text) = clipboard.get_text() {
        return Ok(ClipboardContent::Text(text));
    }
    if let Ok(img) = clipboard.get_image() {
        let mut buf = Vec::new();
        if let Ok(img) = to_dynamic_image(img) {
            let _ = img.write_to(&mut Cursor::new(&mut buf), output_format.clone())?;
            return Ok(ClipboardContent::Image(buf));
        }
    }
    Ok(ClipboardContent::None)
}

impl<T: From<ClipboardContent>> ClipboardHandler for &mut Watcher<T> {
    fn on_clipboard_change(&mut self) -> CallbackResult {
        eprintln!("Clipboard content changed");
        match get_clipboard_content(&self.output_format) {
            Ok(content) => {
                self.channel.send(T::from(content)).unwrap();
            }
            Err(err) => {
                eprintln!("Error: {}", err);
            }
        }
        CallbackResult::Next
    }

    fn on_clipboard_error(&mut self, error: io::Error) -> CallbackResult {
        eprintln!("Error: {}", error);
        CallbackResult::Next
    }
}

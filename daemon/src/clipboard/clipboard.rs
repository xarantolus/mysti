use anyhow::Context;
use anyhow::Result;
use clipboard_master::{CallbackResult, ClipboardHandler, Master};
use common::ClipboardContent;
use image::GenericImageView;
use image::ImageOutputFormat;
use image::RgbaImage;
use std::io;
use std::io::Cursor;
use tokio::sync::mpsc::Sender;

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

fn from_dynamic_image(image: DynamicImage) -> Result<ImageData<'static>> {
    let (width, height) = image.dimensions();
    let bytes = image.to_rgba8().into_raw();
    Ok(ImageData {
        width: width as usize,
        height: height as usize,
        bytes: bytes.into(),
    })
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
        log::info!("Clipboard content changed");
        match get_clipboard_content(&self.output_format) {
            Ok(content) => {
                // Since we cannot make this function async, use a trick to send the content
                // to the main thread
                let _ = self.channel.try_send(content.into());
            }
            Err(err) => {
                log::warn!("Error: {}", err);
            }
        }
        CallbackResult::Next
    }

    fn on_clipboard_error(&mut self, error: io::Error) -> CallbackResult {
        log::warn!("Error: {}", error);
        CallbackResult::Next
    }
}

pub fn set_clipboard(content: &ClipboardContent) -> anyhow::Result<()> {
    match &content {
        ClipboardContent::None => Ok(()),
        ClipboardContent::Text(text) => {
            let mut clipboard = Clipboard::new()?;

            // Check the current text and only set if it's different
            if let Ok(current_text) = clipboard.get_text() {
                if current_text == *text {
                    return Ok(());
                }
            }

            clipboard
                .set_text(text.clone())
                .context("failed to set clipboard text")
        }
        ClipboardContent::Image(bytes) => {
            let mut clipboard = Clipboard::new()?;
            let img = image::load_from_memory(bytes)?;
            let clipboard_image = from_dynamic_image(img)?;

            // Check the current image and only set if it's different
            if let Ok(current_image) = clipboard.get_image() {
                if current_image.height == clipboard_image.height
                    && current_image.width == clipboard_image.width
                    && current_image.bytes == clipboard_image.bytes
                {
                    return Ok(());
                }
            }

            clipboard
                .set_image(clipboard_image)
                .context("failed to set clipboard image")
        }
    }
}

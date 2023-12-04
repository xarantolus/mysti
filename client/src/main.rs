use anyhow::Context;
use clipboard_master::{Master, ClipboardHandler, CallbackResult};
use anyhow::Result;
use image::ImageOutputFormat;
use image::RgbaImage;
use std::io;
use std::io::Cursor;

use arboard::Clipboard;
use arboard::ImageData;
use image::DynamicImage;
use image::ImageOutputFormat::{Bmp, Png, Jpeg};


struct Handler(
    Box<dyn FnMut()>,
    ImageOutputFormat,
);

impl ClipboardHandler for Handler {
    fn on_clipboard_change(&mut self) -> CallbackResult {
        eprintln!("Clipboard content changed");
        (self.0)();
        CallbackResult::Next
    }

    fn on_clipboard_error(&mut self, error: io::Error) -> CallbackResult {
        eprintln!("Error: {}", error);
        CallbackResult::Next
    }
}

enum ClipboardContent {
    Text(String),
    Image(Vec<u8>, ImageOutputFormat),
    None,
}

fn to_dynamic_image(image: ImageData) -> Result<DynamicImage> {
    Ok(DynamicImage::ImageRgba8(RgbaImage::from_raw(image.width as u32, image.height as u32, image.bytes.into_owned()).context("failed to decode image")?))
}

fn get_clipboard_content() -> Result<ClipboardContent> {
    let mut clipboard = Clipboard::new()?;
    if let Ok(text) = clipboard.get_text() {
        return Ok(ClipboardContent::Text(text));
    }
    if let Ok(img) = clipboard.get_image() {
        let mut buf = Vec::new();
        if let Ok(img) = to_dynamic_image(img) {
            const OUTPUT_FORMAT: ImageOutputFormat = Bmp;
            let _ = img.write_to(&mut Cursor::new(&mut buf), OUTPUT_FORMAT)?;
            return Ok(ClipboardContent::Image(buf, OUTPUT_FORMAT));
        }
    }
    Ok(ClipboardContent::None)
}

fn main() {
    let handler = || {
        match get_clipboard_content() {
            Ok(ClipboardContent::Text(text)) => {
                println!("Text: {}", text);
            }
            Ok(ClipboardContent::Image(image, _)) => {
                println!("Image: {} bytes", image.len());
            }
            Ok(ClipboardContent::None) => {
                println!("None");
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    };

    let _ = Master::new(Handler(Box::new(handler))).run();
}

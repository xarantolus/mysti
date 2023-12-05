#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    ClipboardText(String),
    ClipboardImage(Vec<u8>),
}

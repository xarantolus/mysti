#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectedClientInfo {
    pub name: String,
    pub id: usize,
    pub connected_at: std::time::SystemTime,
}

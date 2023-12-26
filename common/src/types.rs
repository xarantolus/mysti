use std::{fmt::{Display, Formatter}, time::SystemTime};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectedClientInfo {
    pub name: String,
    pub id: usize,
    pub connected_at: std::time::SystemTime,
}

impl Display for ConnectedClientInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (id: {}, connected {}s ago)",
            self.name,
            self.id,
            SystemTime::now()
                .duration_since(self.connected_at)
                .unwrap()
                .as_secs()
        )
    }
}

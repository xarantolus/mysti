use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionWrapper {
    pub action: Action,
    #[serde(default = "Vec::new")]
    pub args: Vec<String>,
}

impl Action {
    pub async fn run(&self) -> Result<()> {
        match self {
            Action::Shutdown => shutdown(),
        }
    }
}

fn shutdown() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;

        Command::new("shutdown")
            .arg("-h")
            .arg("now")
            .spawn()
            .context("Failed to shutdown")?;

        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        Command::new("shutdown")
            .arg("/s")
            .arg("/f")
            .arg("/t")
            .arg("0")
            .spawn()
            .context("Failed to shutdown")?;

        Ok(())
    }
}

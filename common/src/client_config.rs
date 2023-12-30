use anyhow::{Context, Result};
use serde::Deserialize;

use crate::action::ActionDefinition;

#[derive(Deserialize, Debug, Clone)]
pub struct ClientConfig {
    pub server_host: String,
    pub token: String,

    pub wol_shortcut: Option<String>,

    #[serde(default = "Vec::new", rename = "action")]
    pub actions: Vec<ActionDefinition>,
}

pub fn parse_file(name: &str) -> Result<ClientConfig> {
    let contents = std::fs::read_to_string(name).context("Failed to read config file")?;

    parse(&contents)
}

pub fn parse(content: &str) -> Result<ClientConfig> {
    let res = toml::from_str::<ClientConfig>(content)?;

    // make sure no tasks with same name exist
    let mut names = std::collections::HashSet::new();
    for action in &res.actions {
        if !names.insert(&action.name) {
            return Err(anyhow::anyhow!("Duplicate action name {}", action.name));
        }
    }

    Ok(res)
}

/// Look for the configuration file in common directories
/// and stop when finding the first
pub fn find_parse_config() -> Result<ClientConfig> {
    // Search in different order depending on the OS
    // Linux/Mac: XDG_CONFIG_HOME, $HOME/.config, working directory
    // Windows: %USERPROFILE%\.config, working directory

    let mut paths = vec!["mysti.toml".to_string(), "../mysti.toml".to_string()];

    #[cfg(target_os = "windows")]
    {
        if let Some(home) = std::env::var_os("USERPROFILE") {
            if let Ok(home) = home.into_string() {
                paths.push(home + "/.config/mysti.toml");
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(home) = std::env::var_os("XDG_CONFIG_HOME") {
            if let Ok(home) = home.into_string() {
                paths.push(home + "/mysti.toml");
            }
        }

        if let Some(home) = std::env::var_os("HOME") {
            if let Ok(home) = home.into_string() {
                paths.push(home + "/.config/mysti.toml");
            }
        }
    }

    for path in &paths {
        log::debug!("Trying to parse config file {}", path);

        match parse_file(&path) {
            Ok(config) => return Ok(config),
            Err(e) => {
                // Only log if the file exists
                if std::path::Path::new(&path).exists() {
                    log::warn!("Failed to parse config file {}: {}", path, e);
                }
            }
        }
    }

    Err(anyhow::anyhow!(format!(
        "No working config file found in {:?}",
        paths
    )))
}

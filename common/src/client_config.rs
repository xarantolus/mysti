use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct ClientConfig {
    pub server_host: String,
    pub token: String,
    // TODO: Configure actions instead of hardcoding them
}

pub fn parse_file(name: &str) -> Result<ClientConfig> {
    let contents = std::fs::read_to_string(name).context("Failed to read config file")?;

    parse(&contents)
}

pub fn parse(content: &str) -> Result<ClientConfig> {
    toml::from_str::<ClientConfig>(content).context("Error during parse")
}

/// Look for the configuration file in common directories
/// and stop when finding the first
pub fn find_parse_config() -> Result<ClientConfig> {
    // Search in different order depending on the OS
    // Linux/Mac: XDG_CONFIG_HOME, $HOME/.config, working directory
    // Windows: %USERPROFILE%\.config, working directory

    let mut paths = vec![];
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

    paths.push("mysti.toml".to_string());

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

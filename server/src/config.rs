use std::net::IpAddr;

use anyhow::{Context, Result};
use macaddr::MacAddr6;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub web_port: u16,
    pub wake_on_lan: WakeOnLanConfig,
    pub token: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct WakeOnLanConfig {
    pub target_addr: MacAddr6,
    pub router_addr: Option<IpAddr>,
}

pub fn parse_file(name: &str) -> Result<Config> {
    let contents = std::fs::read_to_string(name).context("Failed to read config file")?;

    let config: Config = toml::from_str(&contents).context("Error during parse")?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    // parse normal config file to see if it works
    #[test]
    fn parse_config() {
        let _ = super::parse_file("config.toml").unwrap();
    }
}

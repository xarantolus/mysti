use std::{borrow::Cow, net::IpAddr, str::FromStr};

use anyhow::{Context, Result};
use serde::Deserialize;
use wol::MacAddr;

// web_port = 8080

// [wake_on_lan]
// target_addr = "50-EB-F6-7F-3D-84"
// router_addr = "255.255.255.255:9"

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub web_port: u16,
    pub wake_on_lan: WakeOnLanConfig,
}

#[derive(Deserialize, Debug, Clone)]
pub struct WakeOnLanConfig {
    #[serde(deserialize_with = "deserialize_mac_addr")]
    pub target_addr: MacAddr,
    pub router_addr: Option<IpAddr>,
}

pub fn parse_file(name: &str) -> Result<Config> {
    let contents = std::fs::read_to_string(name).context("Failed to read config file")?;

    let config: Config = toml::from_str(&contents).context("Error during parse")?;

    Ok(config)
}

fn deserialize_mac_addr<'de, D>(deserializer: D) -> Result<MacAddr, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Cow<str> = serde::Deserialize::deserialize(deserializer)?;
    MacAddr::from_str(&s)
        .map_err(|e| serde::de::Error::custom(format!("Failed to parse MAC address: {}", e)))
}

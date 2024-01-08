use std::{net::IpAddr, str::FromStr};

use anyhow::{Context, Result};
use macaddr::MacAddr6;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub web_port: u16,
    pub wake_on_lan: WakeOnLanConfig,
    pub token: String,

    #[serde(default = "Vec::new", rename = "clipboard_action")]
    pub clipboard_actions: Vec<ClipboardAction>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct WakeOnLanConfig {
    pub target_addr: ParseableMacAddr,
    pub router_addr: Option<IpAddr>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ClipboardAction {
    // If the clipboard matches a regex, then the action is triggered
    pub regex: String,

    #[serde(skip)]
    pub(crate) compiled_regex: Option<regex::Regex>,

    // The action to trigger
    pub command: String,
}

pub fn parse_file(name: &str) -> Result<Config> {
    let contents = std::fs::read_to_string(name).context("Failed to read config file")?;

    parse(&contents)
}

pub fn parse(content: &str) -> Result<Config> {
    let mut config: Config = toml::from_str(content).context("Error during parse")?;

    for action in config.clipboard_actions.iter_mut() {
        action.compiled_regex = Some(regex::Regex::new(&action.regex)?);
    }

    Ok(config)
}

#[derive(Debug, Clone)]
pub struct ParseableMacAddr(pub MacAddr6);

impl std::fmt::Display for ParseableMacAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<'de> serde::Deserialize<'de> for ParseableMacAddr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use macaddr::ParseError;
        use serde::de::Error;

        let mac = String::deserialize(deserializer).and_then(|s| {
            MacAddr6::from_str(&s).map_err(|e| match e {
                ParseError::InvalidLength(_) => Error::invalid_length(s.len(), &"17"),
                ParseError::InvalidCharacter(c, _) => {
                    Error::invalid_value(serde::de::Unexpected::Char(c), &"a valid hex character")
                }
            })
        })?;

        Ok(ParseableMacAddr(mac))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn assert_config(config_str: &str) {
        let config = parse(config_str).unwrap();

        assert_eq!(config.web_port, 9138);
        assert_eq!(config.token, "some_token");

        assert_eq!(
            config.wake_on_lan.target_addr.0,
            MacAddr6::new(0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA)
        );
        assert_eq!(
            config.wake_on_lan.router_addr.unwrap(),
            IpAddr::from_str("255.255.255.255").unwrap()
        );
    }

    #[test]
    fn parse_cfg() {
        let config_str = r#"
    web_port = 9138
    token = "some_token"

    [wake_on_lan]
    target_addr = "AA-aa-AA-AA-aa-AA"
    router_addr = "255.255.255.255""#;

        assert_config(config_str);
    }

    #[test]
    fn parse_cfg_different_mac_format() {
        let config_str = r#"
    web_port = 9138
    token = "some_token"

    [wake_on_lan]
    target_addr = "AA:AA:AA:aa:AA:AA"
    router_addr = "255.255.255.255""#;

        assert_config(config_str);
    }
}

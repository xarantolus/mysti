use crate::client_config::ClientConfig;
use anyhow::{Context, Result};
use url::Url;

pub enum Scheme {
    WebSocket,
    HTTP
}

impl Scheme {
    fn get_matching_ws_scheme(&self, current_scheme: &str) -> Result<&str> {
        Ok(match current_scheme {
             "http" => "ws",
             "https" => "wss",
            "ws" => "ws",
            "wss" => "wss",
            "" => "wss",
            _ => {
                return Err(anyhow::anyhow!(
                    "Invalid URL scheme {:?}",
                    current_scheme
                ))
            }
        })
    }

    fn get_matching_http_scheme(&self, current_scheme: &str) -> Result<&str> {
        Ok(match current_scheme {
             "http" => "http",
             "https" => "https",
            "ws" => "http",
            "wss" => "https",
            "" => "https",
            _ => {
                return Err(anyhow::anyhow!(
                    "Invalid URL scheme {:?}",
                    current_scheme
                ))
            }
        })
    }

    pub fn get_matching_scheme(&self, current_scheme: &str) -> Result<&str> {
        match self {
            Scheme::WebSocket => self.get_matching_ws_scheme(current_scheme),
            Scheme::HTTP => self.get_matching_http_scheme(current_scheme),
        }
    }
}

pub fn generate_request_url(cfg: &ClientConfig, path: &str, scheme: Scheme) -> Result<Url> {
    let mut server_url = Url::parse(&cfg.server_host).context("Failed to parse server URL")?;

    server_url.set_path(path);

    server_url
        .set_scheme(
            scheme
                .get_matching_scheme(server_url.scheme())
                .context("Failed to get matching scheme")?,
        )
        .map_err(|_| anyhow::anyhow!("Failed to set scheme"))?;

    server_url
        .query_pairs_mut()
        .append_pair("token", &cfg.token)
        .append_pair("device_name", &crate::name::client_name());

    Ok(server_url)
}

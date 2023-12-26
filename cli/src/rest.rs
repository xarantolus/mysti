use common::{client_config::ClientConfig, types::ConnectedClientInfo, url::generate_request_url};

use serde::de::DeserializeOwned;

fn fetch_and_decode_json<T>(url: url::Url) -> anyhow::Result<T>
where
    T: DeserializeOwned,
{
    let response = reqwest::blocking::get(url)?;

    if response.status().is_success() {
        // Try to deserialize the JSON response
        let data: T = response.json()?;
        Ok(data)
    } else {
        Err(anyhow::anyhow!(
            "Request failed with status code: {}",
            response.status()
        ))
    }
}

pub fn fetch_connected_clients(cfg: &ClientConfig) -> anyhow::Result<Vec<ConnectedClientInfo>> {
    fetch_and_decode_json(generate_request_url(
        cfg,
        "/devices",
        common::url::Scheme::HTTP,
    )?)
}

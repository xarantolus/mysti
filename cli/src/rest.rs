use common::{
    action::Action, client_config::ClientConfig, types::ConnectedClientInfo,
    url::generate_request_url,
};

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

pub fn post_action(cfg: &ClientConfig, client_id: usize, action: &Action) -> anyhow::Result<()> {
    let url = generate_request_url(
        cfg,
        &format!("/actions/create/{}", client_id),
        common::url::Scheme::HTTP,
    )?;

    let client = reqwest::blocking::Client::new();
    let response = client.post(url).json(action).send()?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Request failed with status code: {}",
            response.status()
        ))
    }
}

pub fn send_wol(cfg: &ClientConfig) -> anyhow::Result<()> {
    let url = generate_request_url(
        cfg,
        "/wol",
        common::url::Scheme::HTTP,
    )?;

    let client = reqwest::blocking::Client::new();
    let response = client.post(url).send()?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Request failed with status code: {}",
            response.status()
        ))
    }
}

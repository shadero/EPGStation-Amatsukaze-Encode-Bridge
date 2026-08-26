use std::time::Duration;

use anyhow::{bail, Context, Result};
use reqwest::{Client, Url};
use serde::{de::DeserializeOwned, Serialize};

use super::model::QueueResponse;
use crate::config::Config;

pub(crate) struct AmatsukazeClient {
    base_url: Url,
    http: Client,
    request_timeout: Duration,
    pub(super) poll_interval: Duration,
    pub(super) encode_timeout: Duration,
}

impl AmatsukazeClient {
    pub(crate) fn new(config: &Config, http: Client) -> Result<Self> {
        Ok(Self {
            base_url: base_url(&config.amatsukaze_url)?,
            http,
            request_timeout: Duration::from_secs(config.request_timeout_seconds),
            poll_interval: Duration::from_secs(config.poll_interval_seconds),
            encode_timeout: Duration::from_secs(config.encode_timeout_seconds),
        })
    }

    pub(crate) async fn verify_health(&self) -> Result<()> {
        let health: serde_json::Value = self.get_json("api/health").await?;
        anyhow::ensure!(
            health.get("ok").and_then(|value| value.as_bool()) == Some(true),
            "Amatsukaze health check failed"
        );
        Ok(())
    }

    pub(super) async fn queue(&self) -> Result<QueueResponse> {
        self.get_json("api/queue").await
    }

    pub(super) async fn get_json<T: DeserializeOwned>(&self, relative: &str) -> Result<T> {
        let url = self.endpoint(relative)?;
        let response = self
            .http
            .get(url.clone())
            .timeout(self.request_timeout)
            .send()
            .await
            .with_context(|| format!("GET {url} could not be sent"))?;
        parse_json_response(response, "GET", &url).await
    }

    pub(super) async fn post_json<T: DeserializeOwned, B: Serialize + ?Sized>(
        &self,
        relative: &str,
        body: &B,
    ) -> Result<T> {
        let url = self.endpoint(relative)?;
        let response = self
            .http
            .post(url.clone())
            .json(body)
            .timeout(self.request_timeout)
            .send()
            .await
            .with_context(|| format!("POST {url} could not be sent"))?;
        parse_json_response(response, "POST", &url).await
    }

    fn endpoint(&self, relative: &str) -> Result<Url> {
        self.base_url
            .join(relative)
            .with_context(|| format!("failed to join URL {} with {relative}", self.base_url))
    }
}

async fn parse_json_response<T: DeserializeOwned>(
    response: reqwest::Response,
    method: &str,
    url: &Url,
) -> Result<T> {
    let status = response.status();
    let body = response
        .bytes()
        .await
        .with_context(|| format!("could not read {method} {url} response body (HTTP {status})"))?;
    if !status.is_success() {
        bail!(
            "{method} {url} returned HTTP {status}; response body: {}",
            body_excerpt(&body)
        );
    }
    serde_json::from_slice(&body).with_context(|| {
        format!(
            "invalid JSON from {method} {url} (HTTP {status}); response body: {}",
            body_excerpt(&body)
        )
    })
}

fn body_excerpt(body: &[u8]) -> String {
    const LIMIT: usize = 2_000;
    let text = String::from_utf8_lossy(body);
    if text.chars().count() <= LIMIT {
        text.into_owned()
    } else {
        format!(
            "{}... (truncated)",
            text.chars().take(LIMIT).collect::<String>()
        )
    }
}

fn base_url(value: &str) -> Result<Url> {
    let mut url = Url::parse(value)?;
    if !url.path().ends_with('/') {
        url.set_path(&format!("{}/", url.path()));
    }
    Ok(url)
}

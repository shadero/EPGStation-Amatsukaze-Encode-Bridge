use std::time::Duration;

use anyhow::{Context, Result};
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
            .await?
            .error_for_status()?;
        response
            .json()
            .await
            .with_context(|| format!("invalid JSON from {url}"))
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
            .await?
            .error_for_status()?;
        response
            .json()
            .await
            .with_context(|| format!("invalid JSON from {url}"))
    }

    fn endpoint(&self, relative: &str) -> Result<Url> {
        self.base_url
            .join(relative)
            .with_context(|| format!("failed to join URL {} with {relative}", self.base_url))
    }
}

fn base_url(value: &str) -> Result<Url> {
    let mut url = Url::parse(value)?;
    if !url.path().ends_with('/') {
        url.set_path(&format!("{}/", url.path()));
    }
    Ok(url)
}

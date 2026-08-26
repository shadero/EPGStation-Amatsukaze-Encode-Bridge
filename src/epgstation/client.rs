use std::time::Duration;

use anyhow::{bail, Context, Result};
use reqwest::{Client, Url};
use serde::de::DeserializeOwned;
use tracing::info;

use super::model::{InputVideo, RecordedResponse};
use crate::config::Config;

pub(crate) struct EpgStationClient {
    pub(super) base_url: Url,
    pub(super) http: Client,
    pub(super) request_timeout: Duration,
    pub(super) parent_directory_name: String,
}

impl EpgStationClient {
    pub(crate) fn new(config: &Config, http: Client) -> Result<Self> {
        Ok(Self {
            base_url: base_url(&config.epgstation_url)?,
            http,
            request_timeout: Duration::from_secs(config.request_timeout_seconds),
            parent_directory_name: config.parent_directory_name.clone(),
        })
    }

    pub(crate) async fn resolve_input(
        &self,
        recorded_id: u64,
        input_filename: &str,
    ) -> Result<InputVideo> {
        let recorded = self.get_recorded(recorded_id).await?;
        let matches: Vec<_> = recorded
            .video_files
            .into_iter()
            .filter(|file| file.filename == input_filename)
            .collect();
        if matches.len() != 1 {
            bail!(
                "expected one input video named {input_filename:?}, found {}",
                matches.len()
            );
        }
        let video = matches.into_iter().next().expect("one match");
        anyhow::ensure!(video.size > 0, "input video is empty");
        info!(
            recorded_id,
            video_file_id = video.id,
            filename = input_filename,
            "input resolved"
        );
        Ok(InputVideo {
            id: video.id,
            size: video.size,
        })
    }

    pub(super) async fn get_recorded(&self, recorded_id: u64) -> Result<RecordedResponse> {
        let mut url = self.endpoint(&format!("api/recorded/{recorded_id}"))?;
        url.query_pairs_mut().append_pair("isHalfWidth", "false");
        self.get_json(url).await
    }

    async fn get_json<T: DeserializeOwned>(&self, url: Url) -> Result<T> {
        let response = self
            .http
            .get(url.clone())
            .timeout(self.request_timeout)
            .send()
            .await
            .with_context(|| format!("GET {url} could not be sent"))?;
        let status = response.status();
        let body = response
            .bytes()
            .await
            .with_context(|| format!("could not read GET {url} response body (HTTP {status})"))?;
        if !status.is_success() {
            bail!(
                "GET {url} returned HTTP {status}; response body: {}",
                body_excerpt(&body)
            );
        }
        serde_json::from_slice(&body).with_context(|| {
            format!(
                "invalid JSON from GET {url} (HTTP {status}); response body: {}",
                body_excerpt(&body)
            )
        })
    }

    pub(super) fn endpoint(&self, relative: &str) -> Result<Url> {
        self.base_url
            .join(relative)
            .with_context(|| format!("failed to join URL {} with {relative}", self.base_url))
    }
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

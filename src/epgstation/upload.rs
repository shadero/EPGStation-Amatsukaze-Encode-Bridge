use std::path::Path;

use anyhow::{bail, Context, Result};
use reqwest::multipart;
use tracing::info;

use super::client::EpgStationClient;

impl EpgStationClient {
    pub(crate) async fn upload(
        &self,
        recorded_id: u64,
        output_path: &Path,
        sub_directory: &str,
        view_name: &str,
    ) -> Result<()> {
        let filename = output_path
            .file_name()
            .and_then(|value| value.to_str())
            .context("output filename is not UTF-8")?;
        self.send_upload(recorded_id, output_path, filename, sub_directory, view_name)
            .await
    }

    async fn send_upload(
        &self,
        recorded_id: u64,
        output_path: &Path,
        filename: &str,
        sub_directory: &str,
        view_name: &str,
    ) -> Result<()> {
        let file_part = multipart::Part::file(output_path)
            .await
            .with_context(|| format!("could not open upload file {}", output_path.display()))?
            .mime_str(output_mime(output_path))
            .with_context(|| format!("invalid MIME type for {}", output_path.display()))?
            .file_name(filename.to_owned());
        let form = multipart::Form::new()
            .text("recordedId", recorded_id.to_string())
            .text("parentDirectoryName", self.parent_directory_name.clone())
            .text("subDirectory", sub_directory.to_owned())
            .text("viewName", view_name.to_owned())
            .text("fileType", "encoded")
            .part("file", file_part);
        let url = self.endpoint("api/videos/upload")?;
        let response = self
            .http
            .post(url.clone())
            .multipart(form)
            .send()
            .await
            .with_context(|| {
                format!("POST {url} could not be sent while uploading {filename:?}")
            })?;
        let status = response.status();
        let response_text = response.text().await.with_context(|| {
            format!("could not read upload response from {url} (HTTP {status})")
        })?;
        if !status.is_success() {
            bail!(
                "EPGStation upload returned HTTP {status} for {filename:?}; response body: {}",
                response_excerpt(&response_text)
            );
        }
        info!(%filename, response = %response_text, "upload request completed");

        let result: serde_json::Value = serde_json::from_str(&response_text)
            .context("EPGStation upload response was not JSON")?;
        anyhow::ensure!(
            result.get("code").and_then(|value| value.as_i64()) == Some(200)
                && result.get("result").and_then(|value| value.as_str()) == Some("ok"),
            "EPGStation rejected upload: {response_text}"
        );
        Ok(())
    }
}

fn response_excerpt(response: &str) -> String {
    const LIMIT: usize = 2_000;
    if response.chars().count() <= LIMIT {
        response.to_owned()
    } else {
        format!(
            "{}... (truncated)",
            response.chars().take(LIMIT).collect::<String>()
        )
    }
}

fn output_mime(path: &Path) -> &'static str {
    match extension(path).as_deref() {
        Some("mkv") => "video/x-matroska",
        Some("mp4") => "video/mp4",
        _ => "application/octet-stream",
    }
}

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
}

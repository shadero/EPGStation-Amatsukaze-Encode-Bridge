use std::path::Path;

use anyhow::{Context, Result};
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
            .await?
            .mime_str(output_mime(output_path))?
            .file_name(filename.to_owned());
        let form = multipart::Form::new()
            .text("recordedId", recorded_id.to_string())
            .text("parentDirectoryName", self.parent_directory_name.clone())
            .text("subDirectory", sub_directory.to_owned())
            .text("viewName", view_name.to_owned())
            .text("fileType", "encoded")
            .part("file", file_part);
        let response = self
            .http
            .post(self.endpoint("api/videos/upload")?)
            .multipart(form)
            .send()
            .await?
            .error_for_status()?;
        let response_text = response.text().await?;
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

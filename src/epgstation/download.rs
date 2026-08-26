use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;
use tracing::info;

use super::{client::EpgStationClient, model::InputVideo};

impl EpgStationClient {
    pub(crate) async fn download(&self, video: &InputVideo, output_path: &Path) -> Result<()> {
        let mut url = self.endpoint(&format!("api/videos/{}", video.id))?;
        url.query_pairs_mut().append_pair("isDownload", "true");
        let temporary_path = partial_path(output_path)?;
        ensure_destination_is_empty(output_path, &temporary_path).await?;

        let response = self
            .http
            .get(url.clone())
            .send()
            .await
            .with_context(|| format!("GET {url} could not be sent"))?
            .error_for_status()
            .with_context(|| {
                format!(
                    "EPGStation rejected download of video file {} from {url}",
                    video.id
                )
            })?;
        let mut stream = response.bytes_stream();
        let mut file = tokio::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary_path)
            .await
            .with_context(|| format!("could not create {}", temporary_path.display()))?;
        let mut received = 0_u64;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.with_context(|| {
                format!(
                    "download stream for video file {} stopped after {received} bytes",
                    video.id
                )
            })?;
            file.write_all(&chunk).await.with_context(|| {
                format!(
                    "could not write to {} after {received} bytes",
                    temporary_path.display()
                )
            })?;
            received += chunk.len() as u64;
        }
        file.flush()
            .await
            .with_context(|| format!("could not flush {}", temporary_path.display()))?;
        drop(file);

        anyhow::ensure!(
            received == video.size,
            "downloaded size mismatch: expected {}, got {received}",
            video.size
        );
        tokio::fs::rename(&temporary_path, output_path)
            .await
            .with_context(|| {
                format!(
                    "could not rename completed download {} to {}",
                    temporary_path.display(),
                    output_path.display()
                )
            })?;
        info!(bytes = received, path = %output_path.display(), "download complete");
        Ok(())
    }
}

fn partial_path(output_path: &Path) -> Result<PathBuf> {
    let filename = output_path
        .file_name()
        .and_then(|name| name.to_str())
        .context("input filename is not UTF-8")?;
    Ok(output_path.with_file_name(format!("{filename}.part")))
}

async fn ensure_destination_is_empty(output_path: &Path, temporary_path: &Path) -> Result<()> {
    if tokio::fs::try_exists(output_path).await? || tokio::fs::try_exists(temporary_path).await? {
        bail!(
            "download destination already exists: {}",
            output_path.display()
        );
    }
    Ok(())
}

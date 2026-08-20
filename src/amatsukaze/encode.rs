use std::{collections::HashSet, path::Path, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use tokio::time::{sleep, Instant};
use tracing::info;
use uuid::Uuid;

use super::{
    client::AmatsukazeClient,
    model::{EncodedOutput, QueueAddRequest, QueueItem, QueueOutput, QueueTarget, SubmittedEncode},
};
use crate::support::path::{as_utf8, same_path};

const QUEUE_PRIORITY: i32 = 3;

impl AmatsukazeClient {
    pub(crate) async fn submit(
        &self,
        recorded_id: u64,
        workflow_id: Uuid,
        preset: &str,
        input_path: &Path,
        input_dir: &Path,
        output_dir: &Path,
    ) -> Result<SubmittedEncode> {
        let initial_queue = self.queue().await?;
        let previous_ids: HashSet<i64> = initial_queue.items.iter().map(|item| item.id).collect();
        let payload = QueueAddRequest {
            dir_path: as_utf8(input_dir)?,
            targets: vec![QueueTarget {
                path: as_utf8(input_path)?,
            }],
            mode: "AutoBatch",
            outputs: vec![QueueOutput {
                dst_path: as_utf8(output_dir)?,
                profile: preset,
                priority: QUEUE_PRIORITY,
            }],
            request_id: format!("epgstation-recorded-{recorded_id}-{workflow_id}"),
            add_queue_bat: None,
        };
        let _: serde_json::Value = self.post_json("api/queue/add", &payload).await?;

        let queue_item = self.find_submitted_item(&previous_ids, input_path).await?;
        info!(
            %workflow_id,
            queue_item_id = queue_item.id,
            %preset,
            "Amatsukaze workflow submitted"
        );
        Ok(SubmittedEncode {
            queue_item_id: queue_item.id,
        })
    }

    pub(crate) async fn wait_for_completion(&self, queue_item_id: i64) -> Result<()> {
        let deadline = Instant::now() + self.encode_timeout;
        loop {
            let current = self
                .queue()
                .await?
                .items
                .into_iter()
                .find(|item| item.id == queue_item_id)
                .ok_or_else(|| anyhow!("Amatsukaze queue item {queue_item_id} disappeared"))?;
            match current.state.as_str() {
                "Complete" => return Ok(()),
                "Failed" | "PreFailed" | "Canceled" => bail!(
                    "Amatsukaze workflow ended as {} ({})",
                    current.state,
                    current.state_label
                ),
                _ => {}
            }
            if Instant::now() >= deadline {
                bail!("timed out waiting for Amatsukaze queue item {queue_item_id}");
            }
            sleep(self.poll_interval).await;
        }
    }

    pub(crate) async fn locate_output(
        &self,
        input_path: &Path,
        output_dir: &Path,
    ) -> Result<EncodedOutput> {
        let stem = input_path
            .file_stem()
            .and_then(|value| value.to_str())
            .context("input stem is not UTF-8")?;
        let deadline = Instant::now() + Duration::from_secs(60);

        loop {
            let mut matches = Vec::new();
            let mut entries = tokio::fs::read_dir(output_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                let same_stem = path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.eq_ignore_ascii_case(stem));
                if !same_stem || !entry.file_type().await?.is_file() {
                    continue;
                }
                if entry.metadata().await?.len() > 0 {
                    matches.push(path);
                }
            }

            match matches.len() {
                1 => {
                    let path = matches.pop().expect("one output match");
                    let filename = path
                        .file_name()
                        .and_then(|value| value.to_str())
                        .context("output filename is not UTF-8")?
                        .to_owned();
                    return Ok(EncodedOutput { path, filename });
                }
                0 if Instant::now() < deadline => sleep(Duration::from_secs(2)).await,
                0 => bail!(
                    "Amatsukaze did not create a non-empty output named {stem} with any extension"
                ),
                count => bail!(
                    "Amatsukaze created {count} outputs named {stem} with different extensions"
                ),
            }
        }
    }

    async fn find_submitted_item(
        &self,
        previous_ids: &HashSet<i64>,
        input_path: &Path,
    ) -> Result<QueueItem> {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if let Some(item) = self.queue().await?.items.into_iter().find(|item| {
                !previous_ids.contains(&item.id) && same_path(&item.src_path, input_path)
            }) {
                return Ok(item);
            }
            if Instant::now() >= deadline {
                bail!("submitted Amatsukaze queue item was not found");
            }
            sleep(Duration::from_secs(1)).await;
        }
    }
}

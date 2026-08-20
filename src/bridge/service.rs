use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};

use anyhow::Result;
use reqwest::Client;
use tokio::sync::{RwLock, Semaphore};
use uuid::Uuid;

use super::{BridgeRequest, WorkflowStatus};
use crate::{amatsukaze::AmatsukazeClient, config::Config, epgstation::EpgStationClient};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(super) struct RequestKey {
    recorded_id: u64,
    input_filename: String,
    preset: String,
    sub_directory: String,
    view_name: String,
}

impl From<&BridgeRequest> for RequestKey {
    fn from(request: &BridgeRequest) -> Self {
        Self {
            recorded_id: request.recorded_id,
            input_filename: request.input_filename.clone(),
            preset: request.preset.clone(),
            sub_directory: request.sub_directory.clone(),
            view_name: request.view_name.clone(),
        }
    }
}

pub(crate) struct BridgeSubmission {
    pub(crate) workflow_id: Uuid,
    pub(crate) existing: bool,
}

/// Shared services and in-memory state used to run Bridge workflows.
pub(crate) struct BridgeService {
    pub(super) work_dir: PathBuf,
    pub(super) keep_failed_files: bool,
    pub(super) epgstation: EpgStationClient,
    pub(super) amatsukaze: AmatsukazeClient,
    pub(super) workflows: RwLock<HashMap<Uuid, WorkflowStatus>>,
    pub(super) request_index: RwLock<HashMap<RequestKey, Uuid>>,
    pub(super) download_queue: Semaphore,
    pub(super) upload_queue: Semaphore,
}

impl BridgeService {
    pub(crate) fn new(config: Config) -> Result<Self> {
        let http = Client::builder()
            .connect_timeout(Duration::from_secs(config.request_timeout_seconds))
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()?;
        let epgstation = EpgStationClient::new(&config, http.clone())?;
        let amatsukaze = AmatsukazeClient::new(&config, http)?;
        Ok(Self {
            work_dir: PathBuf::from(&config.work_dir),
            keep_failed_files: config.keep_failed_files,
            epgstation,
            amatsukaze,
            workflows: RwLock::new(HashMap::new()),
            request_index: RwLock::new(HashMap::new()),
            download_queue: Semaphore::new(config.download_concurrency),
            upload_queue: Semaphore::new(config.upload_concurrency),
        })
    }

    pub(crate) async fn submit(self: &Arc<Self>, request: BridgeRequest) -> BridgeSubmission {
        let key = RequestKey::from(&request);
        let mut request_index = self.request_index.write().await;
        if let Some(workflow_id) = request_index.get(&key).copied() {
            return BridgeSubmission {
                workflow_id,
                existing: true,
            };
        }

        let workflow_id = Uuid::new_v4();
        self.workflows
            .write()
            .await
            .insert(workflow_id, WorkflowStatus::new(workflow_id, &request));
        request_index.insert(key.clone(), workflow_id);
        drop(request_index);

        let service = self.clone();
        tokio::spawn(async move {
            service.run_workflow(workflow_id, key, request).await;
        });
        BridgeSubmission {
            workflow_id,
            existing: false,
        }
    }

    pub(crate) async fn workflow(&self, workflow_id: Uuid) -> Option<WorkflowStatus> {
        self.workflows.read().await.get(&workflow_id).cloned()
    }
}

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use super::BridgeRequest;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WorkflowState {
    Queued,
    Running,
    Succeeded,
    Failed,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WorkflowStage {
    Accepted,
    ResolvingInput,
    WaitingForDownload,
    Downloading,
    SubmittingToAmatsukaze,
    Encoding,
    LocatingOutput,
    WaitingForUpload,
    Uploading,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowStatus {
    pub(crate) workflow_id: Uuid,
    pub(crate) state: WorkflowState,
    pub(crate) stage: WorkflowStage,
    pub(crate) recorded_id: u64,
    pub(crate) input_filename: String,
    pub(crate) preset: String,
    pub(crate) sub_directory: String,
    pub(crate) view_name: String,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) queue_item_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) output_filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<String>,
}

impl WorkflowStatus {
    pub(crate) fn new(workflow_id: Uuid, request: &BridgeRequest) -> Self {
        let now = Utc::now();
        Self {
            workflow_id,
            state: WorkflowState::Queued,
            stage: WorkflowStage::Accepted,
            recorded_id: request.recorded_id,
            input_filename: request.input_filename.clone(),
            preset: request.preset.clone(),
            sub_directory: request.sub_directory.clone(),
            view_name: request.view_name.clone(),
            created_at: now,
            updated_at: now,
            queue_item_id: None,
            output_filename: None,
            error: None,
        }
    }
}

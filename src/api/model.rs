use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub(super) struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CreateWorkflowResponse {
    pub workflow_id: Uuid,
    pub status_url: String,
    pub existing: bool,
}

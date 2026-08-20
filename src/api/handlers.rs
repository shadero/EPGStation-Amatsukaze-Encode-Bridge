use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use super::{
    error::{ApiError, ApiResult},
    model::{CreateWorkflowResponse, HealthResponse},
};
use crate::bridge::{BridgeRequest, BridgeService, WorkflowStatus};

pub(super) async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

pub(super) async fn create_workflow(
    State(service): State<Arc<BridgeService>>,
    Json(request): Json<BridgeRequest>,
) -> ApiResult<(StatusCode, Json<CreateWorkflowResponse>)> {
    request.validate().map_err(ApiError::bad_request)?;

    let submission = service.submit(request).await;
    let response = CreateWorkflowResponse {
        workflow_id: submission.workflow_id,
        status_url: format!("/workflows/{}", submission.workflow_id),
        existing: submission.existing,
    };
    Ok((StatusCode::ACCEPTED, Json(response)))
}

pub(super) async fn get_workflow(
    State(service): State<Arc<BridgeService>>,
    Path(workflow_id): Path<Uuid>,
) -> ApiResult<Json<WorkflowStatus>> {
    service
        .workflow(workflow_id)
        .await
        .map(Json)
        .ok_or_else(|| ApiError::not_found("workflow not found"))
}

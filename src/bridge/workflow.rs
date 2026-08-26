use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::Utc;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::{
    paths::WorkflowPaths,
    service::{BridgeService, RequestKey},
    status::{WorkflowStage, WorkflowState},
    BridgeRequest,
};

impl BridgeService {
    pub(super) async fn run_workflow(
        self: Arc<Self>,
        workflow_id: Uuid,
        key: RequestKey,
        request: BridgeRequest,
    ) {
        if let Err(workflow_error) = self.process_workflow(workflow_id, &request).await {
            let error_chain = format!("{workflow_error:#}");
            let (failed_at, queue_item_id) =
                self.mark_failed(workflow_id, error_chain.clone()).await;
            error!(
                %workflow_id,
                recorded_id = request.recorded_id,
                input_filename = %request.input_filename,
                preset = %request.preset,
                failed_at = ?failed_at,
                queue_item_id = ?queue_item_id,
                error = %error_chain,
                "workflow failed"
            );
            if !self.keep_failed_files {
                self.remove_workflow_files(workflow_id, "failed").await;
            }
        }
        self.request_index.write().await.remove(&key);
    }

    async fn process_workflow(&self, workflow_id: Uuid, request: &BridgeRequest) -> Result<()> {
        self.amatsukaze
            .verify_health()
            .await
            .context("Amatsukaze health check failed before starting workflow")?;

        let paths = WorkflowPaths::new(&self.work_dir, workflow_id, &request.input_filename)
            .with_context(|| {
                format!(
                    "could not construct work paths for input {:?}",
                    request.input_filename
                )
            })?;
        paths.create().await.with_context(|| {
            format!(
                "could not create workflow directory under {}",
                self.work_dir.display()
            )
        })?;

        self.set_stage(workflow_id, WorkflowStage::ResolvingInput)
            .await;
        let input_video = self
            .epgstation
            .resolve_input(request.recorded_id, &request.input_filename)
            .await
            .with_context(|| {
                format!(
                    "could not resolve EPGStation input {:?} for recorded id {}",
                    request.input_filename, request.recorded_id
                )
            })?;
        self.set_stage(workflow_id, WorkflowStage::WaitingForDownload)
            .await;
        {
            let _download_slot = self
                .download_queue
                .acquire()
                .await
                .context("download queue closed")?;
            self.set_stage(workflow_id, WorkflowStage::Downloading)
                .await;
            self.epgstation
                .download(&input_video, &paths.input_path)
                .await
                .with_context(|| {
                    format!(
                        "could not download EPGStation video file {} to {}",
                        input_video.id,
                        paths.input_path.display()
                    )
                })?;
        }

        self.set_stage(workflow_id, WorkflowStage::SubmittingToAmatsukaze)
            .await;
        let encode = self
            .amatsukaze
            .submit(
                request.recorded_id,
                workflow_id,
                &request.preset,
                &paths.input_path,
                &paths.input_dir,
                &paths.output_dir,
            )
            .await
            .with_context(|| {
                format!(
                    "could not submit {} to Amatsukaze with preset {:?}",
                    paths.input_path.display(),
                    request.preset
                )
            })?;
        self.set_queue_item_id(workflow_id, encode.queue_item_id)
            .await;
        self.set_stage(workflow_id, WorkflowStage::Encoding).await;
        self.amatsukaze
            .wait_for_completion(encode.queue_item_id)
            .await
            .with_context(|| {
                format!(
                    "Amatsukaze queue item {} did not complete successfully",
                    encode.queue_item_id
                )
            })?;

        self.set_stage(workflow_id, WorkflowStage::LocatingOutput)
            .await;
        let output = self
            .amatsukaze
            .locate_output(&paths.input_path, &paths.output_dir)
            .await
            .with_context(|| {
                format!(
                    "could not locate encoded output for {} in {}",
                    paths.input_path.display(),
                    paths.output_dir.display()
                )
            })?;
        self.set_output(workflow_id, &output.filename).await;
        self.set_stage(workflow_id, WorkflowStage::WaitingForUpload)
            .await;
        {
            let _upload_slot = self
                .upload_queue
                .acquire()
                .await
                .context("upload queue closed")?;
            self.set_stage(workflow_id, WorkflowStage::Uploading).await;
            self.epgstation
                .upload(
                    request.recorded_id,
                    &output.path,
                    &request.sub_directory,
                    &request.view_name,
                )
                .await
                .with_context(|| {
                    format!(
                        "could not upload {} to EPGStation recorded id {} (subdirectory {:?}, view {:?})",
                        output.path.display(),
                        request.recorded_id,
                        request.sub_directory,
                        request.view_name
                    )
                })?
        };

        self.mark_succeeded(workflow_id).await;
        self.remove_workflow_files(workflow_id, "successful").await;
        Ok(())
    }

    async fn set_stage(&self, workflow_id: Uuid, stage: WorkflowStage) {
        if let Some(workflow) = self.workflows.write().await.get_mut(&workflow_id) {
            workflow.state = WorkflowState::Running;
            workflow.stage = stage;
            workflow.updated_at = Utc::now();
        }
    }

    async fn set_queue_item_id(&self, workflow_id: Uuid, queue_item_id: i64) {
        if let Some(workflow) = self.workflows.write().await.get_mut(&workflow_id) {
            workflow.queue_item_id = Some(queue_item_id);
            workflow.updated_at = Utc::now();
        }
    }

    async fn set_output(&self, workflow_id: Uuid, filename: &str) {
        if let Some(workflow) = self.workflows.write().await.get_mut(&workflow_id) {
            workflow.output_filename = Some(filename.to_owned());
            workflow.updated_at = Utc::now();
        }
    }

    async fn mark_succeeded(&self, workflow_id: Uuid) {
        if let Some(workflow) = self.workflows.write().await.get_mut(&workflow_id) {
            workflow.state = WorkflowState::Succeeded;
            workflow.stage = WorkflowStage::Completed;
            workflow.updated_at = Utc::now();
        }
        info!(%workflow_id, "workflow succeeded");
    }

    async fn mark_failed(
        &self,
        workflow_id: Uuid,
        message: String,
    ) -> (Option<WorkflowStage>, Option<i64>) {
        if let Some(workflow) = self.workflows.write().await.get_mut(&workflow_id) {
            let failed_at = workflow.stage.clone();
            let queue_item_id = workflow.queue_item_id;
            workflow.state = WorkflowState::Failed;
            workflow.stage = WorkflowStage::Failed;
            workflow.failed_at = Some(failed_at.clone());
            workflow.error = Some(message);
            workflow.updated_at = Utc::now();
            return (Some(failed_at), queue_item_id);
        }
        (None, None)
    }

    async fn remove_workflow_files(&self, workflow_id: Uuid, outcome: &str) {
        let root = self.work_dir.join(workflow_id.to_string());
        if let Err(cleanup_error) = tokio::fs::remove_dir_all(&root).await {
            if cleanup_error.kind() != std::io::ErrorKind::NotFound {
                warn!(
                    %workflow_id,
                    error = %cleanup_error,
                    path = %root.display(),
                    %outcome,
                    "workflow files could not be removed"
                );
            }
        }
    }
}

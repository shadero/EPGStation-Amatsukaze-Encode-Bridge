use std::path::{Path, PathBuf};

use anyhow::Result;
use uuid::Uuid;

pub(super) struct WorkflowPaths {
    pub(super) input_dir: PathBuf,
    pub(super) output_dir: PathBuf,
    pub(super) input_path: PathBuf,
}

impl WorkflowPaths {
    pub(super) fn new(work_dir: &Path, workflow_id: Uuid, input_filename: &str) -> Result<Self> {
        let root = work_dir.join(workflow_id.to_string());
        let input_dir = root.join("input");
        let output_dir = root.join("output");
        let input_path = input_dir.join(input_filename);
        anyhow::ensure!(
            input_path.parent() == Some(input_dir.as_path()),
            "unsafe input filename"
        );
        Ok(Self {
            input_dir,
            output_dir,
            input_path,
        })
    }

    pub(super) async fn create(&self) -> Result<()> {
        tokio::fs::create_dir_all(&self.input_dir).await?;
        tokio::fs::create_dir_all(&self.output_dir).await?;
        Ok(())
    }
}

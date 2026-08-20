use serde::Deserialize;

/// A request to download, encode, and upload one EPGStation recording.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BridgeRequest {
    pub(crate) recorded_id: u64,
    pub(crate) input_filename: String,
    pub(crate) preset: String,
    pub(crate) sub_directory: String,
    pub(crate) view_name: String,
}

impl BridgeRequest {
    pub(crate) fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(self.recorded_id > 0, "recordedId must be greater than 0");
        anyhow::ensure!(!self.preset.trim().is_empty(), "preset must not be empty");
        anyhow::ensure!(
            !self.view_name.trim().is_empty(),
            "viewName must not be empty"
        );
        anyhow::ensure!(
            is_plain_filename(&self.input_filename),
            "inputFilename must be a plain filename"
        );
        Ok(())
    }
}

fn is_plain_filename(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && !value.contains('/')
        && !value.contains('\\')
        && !value.contains('\0')
}

use serde::Deserialize;

#[derive(Clone, Debug)]
pub(crate) struct InputVideo {
    pub id: u64,
    pub size: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct VideoFile {
    pub id: u64,
    pub filename: String,
    pub size: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RecordedResponse {
    pub video_files: Vec<VideoFile>,
}

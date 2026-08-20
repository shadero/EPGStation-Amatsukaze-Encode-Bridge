use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub(crate) struct SubmittedEncode {
    pub(crate) queue_item_id: i64,
}

pub(crate) struct EncodedOutput {
    pub(crate) path: PathBuf,
    pub(crate) filename: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct QueueItem {
    pub id: i64,
    pub src_path: String,
    pub state: String,
    #[serde(default)]
    pub state_label: String,
}

#[derive(Deserialize)]
pub(super) struct QueueResponse {
    pub items: Vec<QueueItem>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct QueueAddRequest<'a> {
    pub dir_path: &'a str,
    pub targets: Vec<QueueTarget<'a>>,
    pub mode: &'static str,
    pub outputs: Vec<QueueOutput<'a>>,
    pub request_id: String,
    pub add_queue_bat: Option<String>,
}

#[derive(Serialize)]
pub(super) struct QueueTarget<'a> {
    pub path: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct QueueOutput<'a> {
    pub dst_path: &'a str,
    pub profile: &'a str,
    pub priority: i32,
}

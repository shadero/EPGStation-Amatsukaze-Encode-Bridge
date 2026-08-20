use std::net::SocketAddr;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub listen: SocketAddr,
    pub epgstation_url: String,
    pub amatsukaze_url: String,
    pub work_dir: String,
    pub parent_directory_name: String,
    pub download_concurrency: usize,
    pub upload_concurrency: usize,
    pub poll_interval_seconds: u64,
    pub encode_timeout_seconds: u64,
    pub request_timeout_seconds: u64,
    pub keep_failed_files: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen: "0.0.0.0:8765"
                .parse()
                .expect("valid default listen address"),
            epgstation_url: "http://127.0.0.1:8888/".to_owned(),
            amatsukaze_url: "http://127.0.0.1:32769/".to_owned(),
            work_dir: "work".to_owned(),
            parent_directory_name: "recorded".to_owned(),
            download_concurrency: 1,
            upload_concurrency: 1,
            poll_interval_seconds: 10,
            encode_timeout_seconds: 10_800,
            request_timeout_seconds: 30,
            keep_failed_files: true,
        }
    }
}

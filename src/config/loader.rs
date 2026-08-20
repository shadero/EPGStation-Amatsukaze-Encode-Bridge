use std::{fs, path::Path};

use anyhow::{Context, Result};

use super::Config;

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)
            .with_context(|| format!("failed to read config: {}", path.display()))?;
        let config: Self = toml::from_str(&text)
            .with_context(|| format!("failed to parse config: {}", path.display()))?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        reqwest::Url::parse(&self.epgstation_url).context("epgstation_url is invalid")?;
        reqwest::Url::parse(&self.amatsukaze_url).context("amatsukaze_url is invalid")?;
        anyhow::ensure!(
            !self.work_dir.trim().is_empty(),
            "work_dir must not be empty"
        );
        anyhow::ensure!(
            !self.parent_directory_name.trim().is_empty(),
            "parent_directory_name must not be empty"
        );
        anyhow::ensure!(
            self.download_concurrency > 0,
            "download_concurrency must be greater than 0"
        );
        anyhow::ensure!(
            self.upload_concurrency > 0,
            "upload_concurrency must be greater than 0"
        );
        anyhow::ensure!(
            self.poll_interval_seconds > 0,
            "poll_interval_seconds must be greater than 0"
        );
        anyhow::ensure!(
            self.encode_timeout_seconds > 0,
            "encode_timeout_seconds must be greater than 0"
        );
        anyhow::ensure!(
            self.request_timeout_seconds > 0,
            "request_timeout_seconds must be greater than 0"
        );
        Ok(())
    }
}

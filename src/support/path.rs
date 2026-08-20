use std::path::Path;

use anyhow::{anyhow, Result};

pub(crate) fn as_utf8(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| anyhow!("path is not UTF-8: {}", path.display()))
}

pub(crate) fn same_path(value: &str, path: &Path) -> bool {
    #[cfg(windows)]
    {
        normalize_windows_path(value) == normalize_windows_path(&path.to_string_lossy())
    }

    #[cfg(not(windows))]
    {
        Path::new(value) == path
    }
}

#[cfg(windows)]
fn normalize_windows_path(value: &str) -> String {
    value.replace('/', "\\").to_lowercase()
}

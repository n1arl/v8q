use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context};

pub fn expand_tilde(value: &str) -> PathBuf {
    if value == "~" {
        return dirs::home_dir().unwrap_or_else(|| PathBuf::from(value));
    }

    if let Some(rest) = value.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }

    PathBuf::from(value)
}

pub fn config_file() -> anyhow::Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow!("could not locate user config directory"))?
        .join("v8q");

    Ok(config_dir.join("config.toml"))
}

pub fn state_dir() -> PathBuf {
    dirs::state_dir()
        .unwrap_or_else(|| {
            dirs::home_dir()
                .map(|home| home.join(".local/state"))
                .unwrap_or_else(std::env::temp_dir)
        })
        .join("v8q")
}

pub fn logs_dir() -> PathBuf {
    state_dir().join("logs")
}

pub fn runtime_dir() -> PathBuf {
    if let Some(value) = std::env::var_os("XDG_RUNTIME_DIR") {
        return PathBuf::from(value).join("v8q");
    }

    let uid = std::env::var("UID").unwrap_or_else(|_| "unknown".to_string());
    PathBuf::from(format!("/tmp/v8q-{uid}"))
}

pub fn default_buffer_dir() -> PathBuf {
    runtime_dir().join("buffer")
}

pub fn default_pid_file() -> PathBuf {
    runtime_dir().join("v8q.pid")
}

pub fn default_save_lock_file() -> PathBuf {
    runtime_dir().join("v8q-save.lock")
}

pub fn log_file_for_backend(backend: &str) -> PathBuf {
    let date = chrono::Local::now().format("%Y-%m-%d");
    logs_dir().join(format!("{backend}_{date}.log"))
}

pub fn latest_log_file() -> PathBuf {
    logs_dir().join("latest.log")
}

pub fn ensure_parent_dir(path: &Path) -> anyhow::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("path has no parent: {}", path.display()))?;

    std::fs::create_dir_all(parent)
        .with_context(|| format!("failed to create directory {}", parent.display()))
}

#[cfg(test)]
mod tests {
    use super::expand_tilde;

    #[test]
    fn leaves_absolute_path_unchanged() {
        assert_eq!(expand_tilde("/tmp/v8q").to_string_lossy(), "/tmp/v8q");
    }

    #[test]
    fn expands_home_relative_path() {
        let path = expand_tilde("~/Videos/V8Q");
        assert!(path.to_string_lossy().ends_with("/Videos/V8Q"));
        assert!(!path.to_string_lossy().starts_with('~'));
    }
}

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockMeta {
    pub pid: u32,
    pub started_at: String,
    pub operation: String,
}

pub struct FileLock {
    path: PathBuf,
}

impl FileLock {
    pub fn acquire(path: &Path, busy_message: &str) -> anyhow::Result<Self> {
        Self::acquire_for(path, "operation", busy_message)
    }

    pub fn acquire_for(path: &Path, operation: &str, busy_message: &str) -> anyhow::Result<Self> {
        if let Some(meta) = read_lock(path)? {
            if !crate::process::is_process_running(meta.pid) {
                let _ = fs::remove_file(path);
            }
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create lock directory {}", parent.display()))?;
        }
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
        {
            Ok(mut file) => {
                let meta = LockMeta {
                    pid: std::process::id(),
                    started_at: chrono::Local::now().to_rfc3339(),
                    operation: operation.to_string(),
                };
                write!(file, "{}", toml::to_string_pretty(&meta)?)
                    .with_context(|| format!("failed to write lock file {}", path.display()))?;
                Ok(Self {
                    path: path.to_path_buf(),
                })
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                Err(anyhow!(busy_message.to_string()))
            }
            Err(error) => {
                Err(error).with_context(|| format!("failed to create lock {}", path.display()))
            }
        }
    }
}

fn read_lock(path: &Path) -> anyhow::Result<Option<LockMeta>> {
    if !path.exists() {
        return Ok(None);
    }
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read lock {}", path.display()))?;
    Ok(toml::from_str(&contents).ok())
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

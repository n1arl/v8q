use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Context};
use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::paths;
use crate::StopResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMeta {
    pub pid: u32,
    pub backend: String,
    pub command: String,
    pub started_at: String,
    #[serde(default)]
    pub config_path: String,
    #[serde(default)]
    pub log_file: String,
    #[serde(default)]
    pub buffer_dir: String,
    #[serde(default)]
    pub output_dir: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history_file: Option<String>,
    #[serde(default)]
    pub binary_version: String,
}

pub fn read_pid(pid_file: &Path) -> anyhow::Result<Option<u32>> {
    if !pid_file.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(pid_file)
        .with_context(|| format!("failed to read PID file {}", pid_file.display()))?;
    let pid = contents
        .trim()
        .parse::<u32>()
        .with_context(|| format!("invalid PID file {}", pid_file.display()))?;

    Ok(Some(pid))
}

pub fn write_pid(pid_file: &Path, pid: u32) -> anyhow::Result<()> {
    paths::ensure_parent_dir(pid_file)?;
    fs::write(pid_file, format!("{pid}\n"))
        .with_context(|| format!("failed to write PID file {}", pid_file.display()))
}

pub fn write_pid_with_meta(pid_file: &Path, meta: ProcessMeta) -> anyhow::Result<()> {
    write_pid(pid_file, meta.pid)?;
    let contents = toml::to_string_pretty(&meta).context("failed to serialize PID metadata")?;
    fs::write(pid_meta_file(pid_file), contents).with_context(|| {
        format!(
            "failed to write PID metadata {}",
            pid_meta_file(pid_file).display()
        )
    })
}

#[allow(clippy::too_many_arguments)]
pub fn new_meta(
    pid: u32,
    backend: &str,
    command: &str,
    buffer_dir: &Path,
    output_dir: &Path,
    history_file: Option<&Path>,
    config_path: &Path,
    log_file: &Path,
) -> ProcessMeta {
    ProcessMeta {
        pid,
        backend: backend.to_string(),
        command: command.to_string(),
        started_at: Local::now().to_rfc3339(),
        config_path: config_path.to_string_lossy().into_owned(),
        log_file: log_file.to_string_lossy().into_owned(),
        buffer_dir: buffer_dir.to_string_lossy().into_owned(),
        output_dir: output_dir.to_string_lossy().into_owned(),
        history_file: history_file.map(|path| path.to_string_lossy().into_owned()),
        binary_version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

pub fn proc_cmdline(pid: u32) -> Option<String> {
    let path = format!("/proc/{pid}/cmdline");
    let bytes = fs::read(path).ok()?;
    if bytes.is_empty() {
        return None;
    }

    let parts: Vec<_> = bytes
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty())
        .map(|part| String::from_utf8_lossy(part).into_owned())
        .collect();

    (!parts.is_empty()).then(|| parts.join(" "))
}

pub fn read_pid_meta(pid_file: &Path) -> anyhow::Result<Option<ProcessMeta>> {
    let meta_file = pid_meta_file(pid_file);
    if !meta_file.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(&meta_file)
        .with_context(|| format!("failed to read PID metadata {}", meta_file.display()))?;
    let meta = toml::from_str(&contents)
        .with_context(|| format!("failed to parse PID metadata {}", meta_file.display()))?;
    Ok(Some(meta))
}

pub fn pid_meta_file(pid_file: &Path) -> PathBuf {
    let mut path = pid_file.as_os_str().to_os_string();
    path.push(".meta");
    PathBuf::from(path)
}

pub fn remove_pid_file(pid_file: &Path) -> anyhow::Result<()> {
    match fs::remove_file(pid_file) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => {
            Err(error).with_context(|| format!("failed to remove PID file {}", pid_file.display()))
        }
    }
}

pub fn remove_pid_files(pid_file: &Path) -> anyhow::Result<()> {
    remove_pid_file(pid_file)?;
    match fs::remove_file(pid_meta_file(pid_file)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| {
            format!(
                "failed to remove PID metadata {}",
                pid_meta_file(pid_file).display()
            )
        }),
    }
}

pub fn is_process_running(pid: u32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn recorder_pid(pid_file: &Path) -> anyhow::Result<Option<u32>> {
    Ok(recorder_pid_checked(pid_file, None)?.0)
}

pub fn recorder_pid_checked(
    pid_file: &Path,
    expected_backend: Option<&str>,
) -> anyhow::Result<(Option<u32>, Vec<String>)> {
    let mut warnings = Vec::new();
    let Some(pid) = read_pid(pid_file)? else {
        return Ok((None, warnings));
    };

    if !is_process_running(pid) {
        warnings.push(format!("stale PID {pid}: process is not running"));
        remove_pid_files(pid_file)?;
        return Ok((None, warnings));
    }

    let Some(cmdline) = proc_cmdline(pid) else {
        warnings.push(format!(
            "stale PID {pid}: could not read /proc/{pid}/cmdline"
        ));
        remove_pid_files(pid_file)?;
        return Ok((None, warnings));
    };

    if !cmdline_looks_like_recorder(&cmdline) {
        warnings.push(format!(
            "stale PID {pid}: unexpected command line: {cmdline}"
        ));
        remove_pid_files(pid_file)?;
        return Ok((None, warnings));
    }

    if let Some(expected) = expected_backend {
        if expected == "wl-screenrec" && !cmdline.contains("wl-screenrec") {
            warnings.push(format!(
                "stale PID {pid}: expected wl-screenrec but command line was: {cmdline}"
            ));
            remove_pid_files(pid_file)?;
            return Ok((None, warnings));
        }
        if (expected == "x11" || expected == "custom") && !cmdline.contains("ffmpeg") {
            warnings.push(format!(
                "stale PID {pid}: expected ffmpeg but command line was: {cmdline}"
            ));
            remove_pid_files(pid_file)?;
            return Ok((None, warnings));
        }
    }

    Ok((Some(pid), warnings))
}

pub fn cmdline_looks_like_recorder(cmdline: &str) -> bool {
    cmdline.contains("wl-screenrec")
        || cmdline.contains("ffmpeg")
        || (cmdline.contains("v8q")
            && cmdline.contains("start")
            && cmdline.contains("--foreground"))
}

pub fn ensure_not_running(pid_file: &Path) -> anyhow::Result<()> {
    if let Some(pid) = recorder_pid(pid_file)? {
        return Err(anyhow!("recorder is already running with PID {pid}"));
    }

    Ok(())
}

pub fn stop_recorder(pid_file: &Path) -> anyhow::Result<StopResult> {
    let Some(pid) = read_pid(pid_file)? else {
        return Ok(StopResult {
            was_running: false,
            pid: None,
        });
    };

    let mut was_running = false;
    if is_process_running(pid) {
        let Some(cmdline) = proc_cmdline(pid) else {
            remove_pid_files(pid_file)?;
            return Ok(StopResult {
                was_running: false,
                pid: Some(pid),
            });
        };
        if !cmdline_looks_like_recorder(&cmdline) {
            remove_pid_files(pid_file)?;
            return Ok(StopResult {
                was_running: false,
                pid: Some(pid),
            });
        }
        was_running = true;
        let status = Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .status()
            .with_context(|| format!("failed to send SIGTERM to PID {pid}"))?;

        if !status.success() {
            was_running = false;
        }
    }

    remove_pid_files(pid_file)?;
    Ok(StopResult {
        was_running,
        pid: Some(pid),
    })
}

pub fn send_signal(pid: u32, signal: &str) -> anyhow::Result<()> {
    let status = Command::new("kill")
        .arg(format!("-{signal}"))
        .arg(pid.to_string())
        .status()
        .with_context(|| format!("failed to send SIG{signal} to PID {pid}"))?;

    if !status.success() {
        return Err(anyhow!("PID {pid} did not accept SIG{signal}"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::cmdline_looks_like_recorder;

    #[test]
    fn recorder_cmdline_validation_rejects_unrelated_processes() {
        assert!(cmdline_looks_like_recorder("wl-screenrec --history 30"));
        assert!(cmdline_looks_like_recorder("ffmpeg -f segment"));
        assert!(cmdline_looks_like_recorder("v8q start --foreground"));
        assert!(!cmdline_looks_like_recorder("sleep 1000"));
        assert!(!cmdline_looks_like_recorder("v8q status"));
    }
}

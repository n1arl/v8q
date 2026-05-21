pub mod cli;
pub mod config;
pub mod doctor;
pub mod error;
pub mod ffmpeg;
pub mod lock;
pub mod paths;
pub mod preset;
pub mod process;
pub mod replay;
pub mod service;
pub mod window;
pub mod wl_screenrec;

use std::path::PathBuf;

pub use config::{CaptureBackend, Config};
pub use doctor::{DoctorCheck, DoctorCheckStatus, DoctorReport};
pub use error::Result;

#[derive(Debug, Clone)]
pub struct StartResult {
    pub pid: u32,
    pub backend: String,
    pub buffer_dir: PathBuf,
    pub log_file: PathBuf,
}

#[derive(Debug, Clone)]
pub struct StopResult {
    pub was_running: bool,
    pub pid: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct SaveResult {
    pub output_file: PathBuf,
    pub duration_seconds: u64,
    pub backend: String,
}

#[derive(Debug, Clone)]
pub struct StatusInfo {
    pub is_running: bool,
    pub pid: Option<u32>,
    pub backend: String,
    pub config_path: PathBuf,
    pub buffer_dir: PathBuf,
    pub output_dir: PathBuf,
    pub replay_duration: u64,
    pub segment_duration: u64,
    pub fps: u32,
    pub encoder: String,
    pub bitrate: String,
    pub audio_enabled: Option<bool>,
    pub segment_count: Option<usize>,
    pub history_file: Option<PathBuf>,
    pub last_log_lines: Vec<(PathBuf, Vec<String>)>,
    pub process_command: Option<String>,
    pub metadata: Option<process::ProcessMeta>,
    pub warnings: Vec<String>,
    pub error: Option<String>,
    pub log_tail: Vec<String>,
    pub expected_history_file: Option<PathBuf>,
    pub history_exists: Option<bool>,
    pub history_size_bytes: Option<u64>,
    pub history_valid: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct CleanResult {
    pub buffer_dir: PathBuf,
    pub removed_files: usize,
}

#[derive(Debug, Clone)]
pub struct LogsResult {
    pub logs: Vec<(PathBuf, Vec<String>)>,
}

#[derive(Debug, Clone)]
pub struct ClipsResult {
    pub clips: Vec<PathBuf>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SaveJson {
    pub output_file: PathBuf,
    pub backend: String,
    pub duration_seconds: u64,
    pub size_bytes: u64,
    pub saved_at: String,
    pub warnings: Vec<String>,
}

pub fn start_recorder(config: &Config) -> Result<StartResult> {
    match config.effective_backend()? {
        CaptureBackend::WlScreenrec => wl_screenrec::start(config),
        CaptureBackend::X11 | CaptureBackend::Custom => ffmpeg::start(config),
    }
}

pub fn load_or_create_default_config() -> Result<Config> {
    Config::load_or_create_default()
}

pub fn stop_recorder(config: &Config) -> Result<StopResult> {
    let result = process::stop_recorder(&config.paths.pid_file_path())?;
    if config.uses_wl_screenrec().unwrap_or(false) {
        let history_file = config.wl_screenrec_history_file();
        if std::fs::metadata(&history_file)
            .map(|metadata| metadata.len() == 0)
            .unwrap_or(false)
        {
            let _ = std::fs::remove_file(history_file);
        }
    }
    Ok(result)
}

pub fn save_replay(config: &Config) -> Result<SaveResult> {
    replay::save(config)
}

pub fn save_replay_with_options(
    config: &Config,
    options: &replay::SaveOptions,
) -> Result<SaveResult> {
    replay::save_with_options(config, options)
}

pub fn get_status(config: &Config) -> Result<StatusInfo> {
    replay::status_info(config)
}

pub fn run_doctor(config: &Config) -> Result<DoctorReport> {
    doctor::run_report(config)
}

pub fn clean_buffer(config: &Config) -> Result<CleanResult> {
    replay::clean(config)
}

pub fn get_logs(config: &Config) -> Result<LogsResult> {
    replay::logs(config)
}

pub fn config_path() -> Result<PathBuf> {
    Config::config_file_path()
}

pub fn save_config(config: &Config) -> Result<()> {
    config::save_config(config)
}

pub fn validate_config(config: &Config) -> Result<()> {
    config::validate_config(config).map(|_| ())
}

pub fn validate_config_detailed(config: &Config) -> Result<Vec<String>> {
    config::validate_config(config)
}

pub fn migrate_config() -> Result<PathBuf> {
    config::migrate_config_file()
}

pub fn list_clips(config: &Config) -> Result<ClipsResult> {
    replay::list_clips(config)
}

pub fn latest_clip(config: &Config) -> Result<Option<PathBuf>> {
    Ok(list_clips(config)?.clips.into_iter().last())
}

pub fn open_output_folder_command(config: &Config) -> std::process::Command {
    let mut command = std::process::Command::new("xdg-open");
    command.arg(config.paths.output_dir_path());
    command
}

pub fn open_path(path: impl Into<PathBuf>) -> Result<()> {
    let path = path.into();
    let status = std::process::Command::new("xdg-open")
        .arg(&path)
        .status()
        .map_err(anyhow::Error::from)?;
    if !status.success() {
        anyhow::bail!("xdg-open failed for {}", path.display());
    }
    Ok(())
}

pub fn open_config_file() -> Result<()> {
    open_path(config_path()?)
}

pub fn sanitize_clip_name(name: &str) -> String {
    let mut sanitized = String::new();
    let mut last_was_sep = false;
    for ch in name.trim().chars() {
        match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => {
                sanitized.push(ch);
                last_was_sep = false;
            }
            '-' | '_' => {
                sanitized.push(ch);
                last_was_sep = false;
            }
            ' ' | '/' | '\\' | '.' => {
                if !last_was_sep {
                    sanitized.push('_');
                    last_was_sep = true;
                }
            }
            _ => {
                if !last_was_sep {
                    sanitized.push('_');
                    last_was_sep = true;
                }
            }
        }
    }
    let sanitized = sanitized.trim_matches('_').to_string();
    sanitized.chars().take(64).collect()
}

pub fn build_clip_filename(name: Option<&str>) -> String {
    let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
    match name.map(sanitize_clip_name).filter(|name| !name.is_empty()) {
        Some(name) => format!("v8q_{timestamp}_{name}.mkv"),
        None => format!("v8q_{timestamp}.mkv"),
    }
}

pub fn notify_replay_saved(config: &Config, output_file: &std::path::Path) {
    if !config.notifications.enabled || !config.notifications.on_save {
        return;
    }

    let _ = std::process::Command::new(&config.notifications.command)
        .arg("V8Q replay saved")
        .arg(output_file.to_string_lossy().to_string())
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_output_folder_command_uses_xdg_open() {
        let config = Config::default();
        let command = open_output_folder_command(&config);
        assert_eq!(command.get_program().to_string_lossy(), "xdg-open");
        assert_eq!(command.get_args().count(), 1);
    }

    #[test]
    fn get_status_returns_structured_info() {
        let config = Config::default();
        let status = get_status(&config).unwrap();
        assert_eq!(status.backend, "wl-screenrec");
        assert_eq!(status.replay_duration, 30);
        assert_eq!(status.fps, 60);
    }

    #[test]
    fn config_path_resolves() {
        let path = config_path().unwrap();
        assert!(path.ends_with("v8q/config.toml"));
    }

    #[test]
    fn latest_clip_returns_none_for_empty_output_dir() {
        let dir = std::env::temp_dir().join(format!("v8q-empty-clips-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut config = Config::default();
        config.paths.output_dir = dir.to_string_lossy().into_owned();
        assert!(latest_clip(&config).unwrap().is_none());
    }

    #[test]
    fn sanitizes_clip_name() {
        assert_eq!(sanitize_clip_name("ace 1v3"), "ace_1v3");
        assert_eq!(sanitize_clip_name("../bad/name"), "bad_name");
    }
}

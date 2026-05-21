use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub recording: RecordingConfig,
    pub paths: PathsConfig,
    #[serde(default)]
    pub capture: Option<CaptureConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture_window: Option<CaptureWindowConfig>,
    #[serde(default)]
    pub ffmpeg: FfmpegConfig,
    #[serde(default)]
    pub wl_screenrec: WlScreenrecConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub notifications: NotificationsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingConfig {
    pub duration_seconds: u64,
    pub segment_seconds: u64,
    pub fps: u32,
    pub width: u32,
    pub height: u32,
    pub encoder: String,
    pub video_bitrate: String,
    pub audio_codec: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    pub buffer_dir: String,
    pub output_dir: String,
    pub pid_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CaptureConfig {
    pub backend: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<CaptureWindowConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CaptureWindowConfig {
    pub enabled: bool,
    pub title: String,
    pub class: String,
    pub address: String,
    pub geometry: String,
    pub follow: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FfmpegConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture_backend: Option<String>,
    pub custom_record_command: String,
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WlScreenrecConfig {
    pub capture_mode: String,
    pub auto_select_focused_output: bool,
    pub output: String,
    pub geometry: String,
    pub audio: bool,
    pub audio_device: String,
    pub audio_backend: String,
    pub ffmpeg_encoder: String,
    pub ffmpeg_encoder_options: String,
    pub bitrate: String,
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub start_minimized: bool,
    pub close_to_tray: bool,
    pub show_notifications: bool,
    pub theme: String,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationsConfig {
    pub enabled: bool,
    pub command: String,
    pub on_save: bool,
    pub on_error: bool,
    pub on_start: bool,
    pub on_stop: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureBackend {
    WlScreenrec,
    X11,
    Custom,
}

impl CaptureBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WlScreenrec => "wl-screenrec",
            Self::X11 => "x11",
            Self::Custom => "custom",
        }
    }

    pub fn is_segmented(self) -> bool {
        matches!(self, Self::X11 | Self::Custom)
    }
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            backend: "wl-screenrec".to_string(),
            window: None,
        }
    }
}

impl Default for WlScreenrecConfig {
    fn default() -> Self {
        Self {
            capture_mode: "output".to_string(),
            auto_select_focused_output: true,
            output: String::new(),
            geometry: String::new(),
            audio: true,
            audio_device: "default".to_string(),
            audio_backend: "pulse".to_string(),
            ffmpeg_encoder: "h264_nvenc".to_string(),
            ffmpeg_encoder_options: "preset=p5".to_string(),
            bitrate: "20M".to_string(),
            extra_args: Vec::new(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            start_minimized: false,
            close_to_tray: false,
            show_notifications: true,
            theme: "system".to_string(),
            mode: "beginner".to_string(),
        }
    }
}

impl Default for NotificationsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            command: "notify-send".to_string(),
            on_save: true,
            on_error: true,
            on_start: false,
            on_stop: false,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            recording: RecordingConfig {
                duration_seconds: 30,
                segment_seconds: 2,
                fps: 60,
                width: 1920,
                height: 1080,
                encoder: "h264_nvenc".to_string(),
                video_bitrate: "20M".to_string(),
                audio_codec: "aac".to_string(),
            },
            paths: PathsConfig {
                buffer_dir: paths::default_buffer_dir().to_string_lossy().into_owned(),
                output_dir: "~/Videos/V8Q".to_string(),
                pid_file: paths::default_pid_file().to_string_lossy().into_owned(),
            },
            capture: Some(CaptureConfig::default()),
            capture_window: None,
            ffmpeg: FfmpegConfig::default(),
            wl_screenrec: WlScreenrecConfig::default(),
            ui: UiConfig::default(),
            notifications: NotificationsConfig::default(),
        }
    }
}

pub fn config_uses_legacy_backend(config: &Config) -> bool {
    config.capture.is_none() && config.ffmpeg.capture_backend.is_some()
}

pub fn validate_config(config: &Config) -> anyhow::Result<Vec<String>> {
    let mut warnings = Vec::new();

    if config.recording.duration_seconds == 0 {
        anyhow::bail!("recording.duration_seconds must be > 0");
    }
    if !(1..=240).contains(&config.recording.fps) {
        anyhow::bail!("recording.fps must be between 1 and 240");
    }
    if config.effective_bitrate()?.trim().is_empty() {
        anyhow::bail!("bitrate must not be empty");
    }
    if config.effective_encoder()?.trim().is_empty() {
        anyhow::bail!("encoder must not be empty");
    }
    if config.effective_backend()? == CaptureBackend::Custom
        && config.ffmpeg.custom_record_command.trim().is_empty()
    {
        anyhow::bail!("backend custom requires ffmpeg.custom_record_command");
    }
    if config.effective_backend()? == CaptureBackend::WlScreenrec
        && !matches!(
            config.wl_screenrec.capture_mode.as_str(),
            "output" | "monitor" | "geometry" | "active-window" | "window"
        )
    {
        anyhow::bail!(
            "wl_screenrec.capture_mode must be one of: output, monitor, geometry, active-window, window"
        );
    }
    if !matches!(config.ui.mode.as_str(), "beginner" | "advanced") {
        anyhow::bail!("ui.mode must be beginner or advanced");
    }
    let capture_window = config.effective_capture_window();
    if capture_window.enabled && capture_window.geometry.trim().is_empty() {
        warnings.push(
            "capture.window is enabled but geometry is empty; run `v8q window select`".to_string(),
        );
    }
    if config.capture_window.is_some() {
        warnings.push(
            "legacy [capture_window] is in use; run `v8q config migrate --write`".to_string(),
        );
    }

    let output_dir = config.paths.output_dir_path();
    let buffer_dir = config.paths.buffer_dir_path();
    if output_dir.as_os_str().is_empty() {
        anyhow::bail!("paths.output_dir must not be empty");
    }
    if buffer_dir.as_os_str().is_empty() {
        anyhow::bail!("paths.buffer_dir must not be empty");
    }

    if config_uses_legacy_backend(config) {
        warnings.push(
            "legacy [ffmpeg].capture_backend is in use; run `v8q config migrate --write`"
                .to_string(),
        );
    }

    Ok(warnings)
}

pub fn migrate_config_file() -> anyhow::Result<PathBuf> {
    let path = Config::config_file_path()?;
    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let raw: toml::Value =
        toml::from_str(&contents).with_context(|| format!("failed to parse {}", path.display()))?;
    let config = migrate_config_value(raw)?;
    let backup = path.with_extension(format!(
        "toml.bak-{}",
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    ));
    std::fs::copy(&path, &backup)
        .with_context(|| format!("failed to create backup {}", backup.display()))?;
    save_config(&config)?;
    Ok(backup)
}

fn migrate_config_value(raw: toml::Value) -> anyhow::Result<Config> {
    let legacy_window = raw.get("capture_window").cloned();
    let nested_window_exists = raw
        .get("capture")
        .and_then(|capture| capture.get("window"))
        .is_some();
    let mut config: Config = raw.try_into().context("failed to parse config")?;
    if config.capture.is_none() {
        let backend = config
            .ffmpeg
            .capture_backend
            .clone()
            .unwrap_or_else(|| "wl-screenrec".to_string());
        config.capture = Some(CaptureConfig {
            backend,
            window: None,
        });
        config.ffmpeg.capture_backend = None;
    }
    if config.paths.buffer_dir_path() == Path::new("/tmp/v8q-buffer") {
        config.paths.buffer_dir = paths::default_buffer_dir().to_string_lossy().into_owned();
    }
    if config.paths.pid_file_path() == Path::new("/tmp/v8q.pid") {
        config.paths.pid_file = paths::default_pid_file().to_string_lossy().into_owned();
    }
    if !nested_window_exists {
        if let Some(window) = legacy_window {
            config
                .capture
                .get_or_insert_with(CaptureConfig::default)
                .window = Some(
                window
                    .try_into()
                    .context("failed to migrate [capture_window]")?,
            );
        }
    }
    config.capture_window = None;
    Ok(config)
}

impl Config {
    pub fn load_or_create_default() -> anyhow::Result<Self> {
        let config_file = paths::config_file()?;

        if !config_file.exists() {
            paths::ensure_parent_dir(&config_file)?;
            let default = Self::default();
            let contents =
                toml::to_string_pretty(&default).context("failed to serialize default config")?;
            std::fs::write(&config_file, contents)
                .with_context(|| format!("failed to write {}", config_file.display()))?;
            return Ok(default);
        }

        let contents = std::fs::read_to_string(&config_file)
            .with_context(|| format!("failed to read {}", config_file.display()))?;
        toml::from_str(&contents)
            .with_context(|| format!("failed to parse {}", config_file.display()))
    }

    pub fn config_file_path() -> anyhow::Result<PathBuf> {
        paths::config_file()
    }

    pub fn replay_segment_count(&self) -> usize {
        let duration = self.recording.duration_seconds.max(1);
        let segment = self.recording.segment_seconds.max(1);
        duration.div_ceil(segment) as usize
    }

    pub fn ffmpeg_segment_wrap_count(&self) -> usize {
        self.replay_segment_count() + 3
    }

    pub fn wl_screenrec_history_file(&self) -> PathBuf {
        self.paths.buffer_dir_path().join("history.mkv")
    }

    pub fn effective_backend_name(&self) -> String {
        if let Some(capture) = &self.capture {
            return capture.backend.clone();
        }

        self.ffmpeg
            .capture_backend
            .clone()
            .unwrap_or_else(|| "wl-screenrec".to_string())
    }

    pub fn effective_capture_window(&self) -> CaptureWindowConfig {
        self.capture
            .as_ref()
            .and_then(|capture| capture.window.clone())
            .or_else(|| self.capture_window.clone())
            .unwrap_or_default()
    }

    pub fn effective_backend(&self) -> anyhow::Result<CaptureBackend> {
        let backend = self.effective_backend_name();
        match backend.as_str() {
            "wl-screenrec" | "hyprland" | "wayland" => Ok(CaptureBackend::WlScreenrec),
            "x11" => Ok(CaptureBackend::X11),
            "custom" => Ok(CaptureBackend::Custom),
            other => Err(anyhow::anyhow!(
                "unknown capture backend '{other}'. Valid backends: wl-screenrec, wayland, hyprland, x11, custom"
            )),
        }
    }

    pub fn uses_wl_screenrec(&self) -> anyhow::Result<bool> {
        Ok(self.effective_backend()? == CaptureBackend::WlScreenrec)
    }

    pub fn effective_encoder(&self) -> anyhow::Result<&str> {
        if self.uses_wl_screenrec()? {
            Ok(&self.wl_screenrec.ffmpeg_encoder)
        } else {
            Ok(&self.recording.encoder)
        }
    }

    pub fn effective_bitrate(&self) -> anyhow::Result<&str> {
        if self.uses_wl_screenrec()? {
            Ok(&self.wl_screenrec.bitrate)
        } else {
            Ok(&self.recording.video_bitrate)
        }
    }
}

pub fn save_config(config: &Config) -> anyhow::Result<()> {
    let config_file = Config::config_file_path()?;
    paths::ensure_parent_dir(&config_file)?;
    let contents = toml::to_string_pretty(config).context("failed to serialize config")?;
    std::fs::write(&config_file, contents)
        .with_context(|| format!("failed to write {}", config_file.display()))
}

impl PathsConfig {
    pub fn buffer_dir_path(&self) -> PathBuf {
        paths::expand_tilde(&self.buffer_dir)
    }

    pub fn output_dir_path(&self) -> PathBuf {
        paths::expand_tilde(&self.output_dir)
    }

    pub fn pid_file_path(&self) -> PathBuf {
        paths::expand_tilde(&self.pid_file)
    }
}

#[cfg(test)]
mod tests {
    use super::{CaptureBackend, Config};

    #[test]
    fn parses_new_capture_backend() {
        let config: Config = toml::from_str(
            r#"
[recording]
duration_seconds = 30
segment_seconds = 2
fps = 60
width = 1920
height = 1080
encoder = "h264_nvenc"
video_bitrate = "20M"
audio_codec = "aac"

[paths]
buffer_dir = "/tmp/v8q-buffer"
output_dir = "~/Videos/V8Q"
pid_file = "/tmp/v8q.pid"

[capture]
backend = "wl-screenrec"

[ffmpeg]
custom_record_command = ""
extra_args = []
"#,
        )
        .unwrap();

        assert_eq!(
            config.effective_backend().unwrap(),
            CaptureBackend::WlScreenrec
        );
    }

    #[test]
    fn parses_legacy_ffmpeg_capture_backend() {
        let config: Config = toml::from_str(
            r#"
[recording]
duration_seconds = 30
segment_seconds = 2
fps = 60
width = 1920
height = 1080
encoder = "h264_nvenc"
video_bitrate = "20M"
audio_codec = "aac"

[paths]
buffer_dir = "/tmp/v8q-buffer"
output_dir = "~/Videos/V8Q"
pid_file = "/tmp/v8q.pid"

[ffmpeg]
capture_backend = "hyprland"
custom_record_command = ""
extra_args = []
"#,
        )
        .unwrap();

        assert_eq!(
            config.effective_backend().unwrap(),
            CaptureBackend::WlScreenrec
        );
    }

    #[test]
    fn serializes_ui_mode_and_window_capture() {
        let mut config = Config::default();
        config.ui.mode = "advanced".to_string();
        config.capture.as_mut().unwrap().window = Some(super::CaptureWindowConfig {
            enabled: true,
            geometry: "1,2 300x400".to_string(),
            ..super::CaptureWindowConfig::default()
        });
        let text = toml::to_string(&config).unwrap();
        assert!(text.contains("mode = \"advanced\""));
        assert!(text.contains("[capture.window]"));
        assert!(text.contains("geometry = \"1,2 300x400\""));
    }

    #[test]
    fn parses_nested_capture_window() {
        let config: Config = toml::from_str(
            r#"
[recording]
duration_seconds = 30
segment_seconds = 2
fps = 60
width = 1920
height = 1080
encoder = "h264_nvenc"
video_bitrate = "20M"
audio_codec = "aac"

[paths]
buffer_dir = "/tmp/v8q-buffer"
output_dir = "~/Videos/V8Q"
pid_file = "/tmp/v8q.pid"

[capture]
backend = "wl-screenrec"

[capture.window]
enabled = true
title = "Firefox"
class = "firefox"
address = "0xabc"
geometry = "1,2 300x400"
follow = false
"#,
        )
        .unwrap();

        let window = config.effective_capture_window();
        assert!(window.enabled);
        assert_eq!(window.geometry, "1,2 300x400");
    }

    #[test]
    fn parses_legacy_capture_window() {
        let config: Config = toml::from_str(
            r#"
[recording]
duration_seconds = 30
segment_seconds = 2
fps = 60
width = 1920
height = 1080
encoder = "h264_nvenc"
video_bitrate = "20M"
audio_codec = "aac"

[paths]
buffer_dir = "/tmp/v8q-buffer"
output_dir = "~/Videos/V8Q"
pid_file = "/tmp/v8q.pid"

[capture]
backend = "wl-screenrec"

[capture_window]
enabled = true
title = "Legacy"
class = "legacy"
address = "0xabc"
geometry = "5,6 700x800"
follow = false
"#,
        )
        .unwrap();

        let window = config.effective_capture_window();
        assert!(window.enabled);
        assert_eq!(window.geometry, "5,6 700x800");
    }

    #[test]
    fn nested_capture_window_wins_over_legacy() {
        let config: Config = toml::from_str(
            r#"
[recording]
duration_seconds = 30
segment_seconds = 2
fps = 60
width = 1920
height = 1080
encoder = "h264_nvenc"
video_bitrate = "20M"
audio_codec = "aac"

[paths]
buffer_dir = "/tmp/v8q-buffer"
output_dir = "~/Videos/V8Q"
pid_file = "/tmp/v8q.pid"

[capture]
backend = "wl-screenrec"

[capture.window]
enabled = true
geometry = "1,2 300x400"

[capture_window]
enabled = true
geometry = "5,6 700x800"
"#,
        )
        .unwrap();

        assert_eq!(config.effective_capture_window().geometry, "1,2 300x400");
    }

    #[test]
    fn migrate_moves_legacy_capture_window_to_nested() {
        let raw: toml::Value = toml::from_str(
            r#"
[recording]
duration_seconds = 30
segment_seconds = 2
fps = 60
width = 1920
height = 1080
encoder = "h264_nvenc"
video_bitrate = "20M"
audio_codec = "aac"

[paths]
buffer_dir = "/tmp/v8q-buffer"
output_dir = "~/Videos/V8Q"
pid_file = "/tmp/v8q.pid"

[capture]
backend = "wl-screenrec"

[capture_window]
enabled = true
geometry = "5,6 700x800"
"#,
        )
        .unwrap();

        let config = super::migrate_config_value(raw).unwrap();
        assert!(config.capture_window.is_none());
        assert_eq!(
            config.capture.unwrap().window.unwrap().geometry,
            "5,6 700x800"
        );
    }

    #[test]
    fn backend_aliases_work() {
        for name in ["wl-screenrec", "hyprland", "wayland"] {
            let mut config = Config::default();
            config.capture.as_mut().unwrap().backend = name.to_string();
            assert_eq!(
                config.effective_backend().unwrap(),
                CaptureBackend::WlScreenrec
            );
        }

        let mut config = Config::default();
        config.capture.as_mut().unwrap().backend = "x11".to_string();
        assert_eq!(config.effective_backend().unwrap(), CaptureBackend::X11);

        config.capture.as_mut().unwrap().backend = "custom".to_string();
        assert_eq!(config.effective_backend().unwrap(), CaptureBackend::Custom);
    }
}

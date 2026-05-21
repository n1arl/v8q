use std::fs::OpenOptions;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Context};

use crate::config::{CaptureBackend, Config};
use crate::{doctor, paths, process, StartResult};

pub fn start(config: &Config) -> anyhow::Result<StartResult> {
    if config.effective_backend()? != CaptureBackend::WlScreenrec {
        return Err(anyhow!(
            "wl-screenrec start called for non-wl-screenrec backend"
        ));
    }

    let buffer_dir = config.paths.buffer_dir_path();
    let output_dir = config.paths.output_dir_path();
    let pid_file = config.paths.pid_file_path();

    process::ensure_not_running(&pid_file)?;
    std::fs::create_dir_all(&buffer_dir)
        .with_context(|| format!("failed to create buffer directory {}", buffer_dir.display()))?;
    std::fs::create_dir_all(&output_dir)
        .with_context(|| format!("failed to create output directory {}", output_dir.display()))?;
    paths::ensure_parent_dir(&pid_file)?;

    let history_file = config.wl_screenrec_history_file();
    let _ = std::fs::remove_file(&history_file);

    let help = validate_wl_screenrec_start()?;
    let args = build_wl_screenrec_command(config, &WlScreenrecCapabilities::from_help(&help))?;
    let (program, program_args) = args
        .split_first()
        .ok_or_else(|| anyhow!("wl-screenrec command is empty"))?;

    let log_file_path = crate::paths::log_file_for_backend("wl-screenrec");
    crate::paths::ensure_parent_dir(&log_file_path)?;
    let stdout = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_file_path)
        .with_context(|| format!("failed to open {}", log_file_path.display()))?;
    let stderr = stdout
        .try_clone()
        .with_context(|| format!("failed to clone {}", log_file_path.display()))?;

    let mut child = Command::new(program)
        .args(program_args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .with_context(|| format!("failed to start wl-screenrec: {}", args.join(" ")))?;

    thread::sleep(Duration::from_millis(1500));
    if let Some(status) = child
        .try_wait()
        .with_context(|| "failed to check wl-screenrec after start")?
    {
        let _ = process::remove_pid_files(&pid_file);
        let log_tail = tail_log(&log_file_path, 80);
        return Err(anyhow!(
            "wl-screenrec exited immediately with status {status}\ncommand: {}\nlog tail:\n{}\n{}",
            args.join(" "),
            log_tail.join("\n"),
            startup_hint(&log_tail)
        ));
    }

    let meta = process::new_meta(
        child.id(),
        "wl-screenrec",
        &args.join(" "),
        &buffer_dir,
        &output_dir,
        Some(&history_file),
        &Config::config_file_path()?,
        &log_file_path,
    );
    process::write_pid_with_meta(&pid_file, meta)?;
    Ok(StartResult {
        pid: child.id(),
        backend: "wl-screenrec".to_string(),
        buffer_dir,
        log_file: log_file_path,
    })
}

fn startup_hint(log_tail: &[String]) -> &'static str {
    if log_tail
        .iter()
        .any(|line| line.contains("multiple enabled displays"))
    {
        "Hint: set `[wl_screenrec] output = \"DP-1\"` or another output from `hyprctl monitors -j`, then rerun `v8q start`."
    } else if log_tail
        .iter()
        .any(|line| line.contains("Failed to negotiate format"))
    {
        "Hint: this wl-screenrec/NVIDIA path failed before recording. Try `[wl_screenrec] extra_args = [\"--experimental-vulkan\"]` for testing, or switch to a custom/wf-recorder backend until wl-screenrec works on this compositor/driver."
    } else {
        "Hint: run `v8q debug wl-screenrec --test-run 5` and inspect the log."
    }
}

fn validate_wl_screenrec_start() -> anyhow::Result<String> {
    if doctor::command_path("wl-screenrec").is_none() {
        return Err(anyhow!(
            "wl-screenrec not found in PATH; install it with `paru -S wl-screenrec`, then rerun `v8q doctor`"
        ));
    }

    let help = doctor::wl_screenrec_help_text()?;
    doctor::wl_screenrec_help_has_flags(&help, &["--history", "--filename"])?;
    Ok(help)
}

pub fn command_for_config(config: &Config) -> anyhow::Result<(Vec<String>, String)> {
    let help = validate_wl_screenrec_start()?;
    let args = build_wl_screenrec_command(config, &WlScreenrecCapabilities::from_help(&help))?;
    Ok((args, help))
}

pub fn log_file_path() -> std::path::PathBuf {
    crate::paths::log_file_for_backend("wl-screenrec")
}

pub fn tail_log(path: &std::path::Path, lines: usize) -> Vec<String> {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    contents
        .lines()
        .rev()
        .take(lines)
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

#[derive(Debug, Clone, Copy)]
pub struct WlScreenrecCapabilities {
    pub max_fps: bool,
    pub ffmpeg_encoder: bool,
    pub audio_codec: bool,
    pub bitrate: bool,
    pub audio: bool,
    pub audio_device: bool,
    pub audio_backend: bool,
    pub ffmpeg_encoder_options: bool,
    pub output: bool,
    pub geometry: bool,
}

impl WlScreenrecCapabilities {
    pub fn from_help(help: &str) -> Self {
        Self {
            max_fps: help.contains("--max-fps"),
            ffmpeg_encoder: help.contains("--ffmpeg-encoder"),
            audio_codec: help.contains("--audio-codec"),
            bitrate: help.contains("--bitrate"),
            audio: help.contains("--audio"),
            audio_device: help.contains("--audio-device"),
            audio_backend: help.contains("--audio-backend"),
            ffmpeg_encoder_options: help.contains("--ffmpeg-encoder-options"),
            output: help.contains("--output"),
            geometry: help.contains("--geometry"),
        }
    }

    #[cfg(test)]
    fn all() -> Self {
        Self {
            max_fps: true,
            ffmpeg_encoder: true,
            audio_codec: true,
            bitrate: true,
            audio: true,
            audio_device: true,
            audio_backend: true,
            ffmpeg_encoder_options: true,
            output: true,
            geometry: true,
        }
    }
}

pub fn build_wl_screenrec_command(
    config: &Config,
    caps: &WlScreenrecCapabilities,
) -> anyhow::Result<Vec<String>> {
    let r = &config.recording;
    let w = &config.wl_screenrec;
    let history_file = config.wl_screenrec_history_file();
    let mut args = vec![
        "wl-screenrec".to_string(),
        "--history".to_string(),
        r.duration_seconds.max(1).to_string(),
        "--filename".to_string(),
        history_file.to_string_lossy().into_owned(),
    ];

    if caps.max_fps {
        args.push("--max-fps".to_string());
        args.push(r.fps.to_string());
    }

    if caps.ffmpeg_encoder {
        args.push("--ffmpeg-encoder".to_string());
        args.push(w.ffmpeg_encoder.clone());
    }

    if caps.bitrate {
        args.push("--bitrate".to_string());
        args.push(bitrate_for_wl_screenrec(&w.bitrate));
    }

    apply_capture_target(config, caps, &mut args)?;

    if w.audio && caps.audio {
        args.push("--audio".to_string());
        if caps.audio_codec {
            args.push("--audio-codec".to_string());
            args.push(r.audio_codec.clone());
        }
        if caps.audio_device && !w.audio_device.trim().is_empty() {
            args.push("--audio-device".to_string());
            args.push(w.audio_device.clone());
        }
        if caps.audio_backend && !w.audio_backend.trim().is_empty() {
            args.push("--audio-backend".to_string());
            args.push(w.audio_backend.clone());
        }
    }

    if caps.ffmpeg_encoder_options && !w.ffmpeg_encoder_options.trim().is_empty() {
        args.push("--ffmpeg-encoder-options".to_string());
        args.push(w.ffmpeg_encoder_options.clone());
    }

    args.extend(w.extra_args.clone());
    Ok(args)
}

fn apply_capture_target(
    config: &Config,
    caps: &WlScreenrecCapabilities,
    args: &mut Vec<String>,
) -> anyhow::Result<()> {
    let w = &config.wl_screenrec;
    let mode = w.capture_mode.as_str();

    if let Some(geometry) = crate::window::selected_window_geometry(config)? {
        if !caps.geometry {
            anyhow::bail!("window capture is enabled, but wl-screenrec --help does not contain --geometry. Update wl-screenrec or use fullscreen capture.");
        }
        args.push("--geometry".to_string());
        args.push(geometry);
        return Ok(());
    }

    if matches!(mode, "geometry" | "active-window" | "window") {
        if !caps.geometry {
            anyhow::bail!("wl-screenrec --help does not contain --geometry");
        }
        let geometry = if matches!(mode, "active-window" | "window") {
            active_window_geometry()?
        } else {
            w.geometry.trim().to_string()
        };
        if geometry.is_empty() {
            anyhow::bail!("wl_screenrec.geometry is empty");
        }
        args.push("--geometry".to_string());
        args.push(geometry);
        return Ok(());
    }

    if !matches!(mode, "output" | "monitor") {
        anyhow::bail!(
            "unknown wl_screenrec.capture_mode '{mode}'. Valid modes: output, monitor, geometry, active-window"
        );
    }

    if !caps.output {
        return Ok(());
    }

    if !w.output.trim().is_empty() {
        args.push("--output".to_string());
        args.push(w.output.clone());
        return Ok(());
    }

    if w.auto_select_focused_output {
        if let Some(output) = focused_hyprland_output() {
            args.push("--output".to_string());
            args.push(output);
        }
    }

    Ok(())
}

fn active_window_geometry() -> anyhow::Result<String> {
    let output = Command::new("hyprctl")
        .args(["activewindow", "-j"])
        .output()
        .context("failed to run hyprctl activewindow -j")?;
    if !output.status.success() {
        anyhow::bail!(
            "hyprctl activewindow -j failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .context("failed to parse hyprctl activewindow -j")?;
    let at = json
        .get("at")
        .and_then(|value| value.as_array())
        .context("hyprctl activewindow did not include at")?;
    let size = json
        .get("size")
        .and_then(|value| value.as_array())
        .context("hyprctl activewindow did not include size")?;
    let x = number_to_i64(at.first()).context("active window x is missing")?;
    let y = number_to_i64(at.get(1)).context("active window y is missing")?;
    let width = number_to_i64(size.first()).context("active window width is missing")?;
    let height = number_to_i64(size.get(1)).context("active window height is missing")?;
    if width <= 0 || height <= 0 {
        anyhow::bail!("active window has invalid size {width}x{height}");
    }
    Ok(format!("{x},{y} {width}x{height}"))
}

fn focused_hyprland_output() -> Option<String> {
    let output = Command::new("hyprctl")
        .args(["monitors", "-j"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let monitors: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    let monitors = monitors.as_array()?;
    monitors
        .iter()
        .find(|monitor| {
            monitor
                .get("focused")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
        })
        .or_else(|| monitors.first())
        .and_then(|monitor| monitor.get("name"))
        .and_then(|name| name.as_str())
        .map(ToString::to_string)
}

fn number_to_i64(value: Option<&serde_json::Value>) -> Option<i64> {
    let value = value?;
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
        .or_else(|| value.as_f64().map(|value| value.round() as i64))
}

fn bitrate_for_wl_screenrec(value: &str) -> String {
    let trimmed = value.trim();
    if let Some(mbits) = trimmed
        .strip_suffix('M')
        .or_else(|| trimmed.strip_suffix('m'))
    {
        if let Ok(mbits) = mbits.trim().parse::<f64>() {
            return format!("{:.0} kB", mbits * 125.0);
        }
    }
    if let Some(kbits) = trimmed
        .strip_suffix('K')
        .or_else(|| trimmed.strip_suffix('k'))
    {
        if let Ok(kbits) = kbits.trim().parse::<f64>() {
            return format!("{:.0} B", kbits * 125.0);
        }
    }
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use crate::config::Config;

    use super::{bitrate_for_wl_screenrec, build_wl_screenrec_command, WlScreenrecCapabilities};

    #[test]
    fn builds_wl_screenrec_history_command() {
        let mut config = Config::default();
        config.capture.as_mut().unwrap().backend = "wl-screenrec".to_string();
        config.wl_screenrec.audio = true;
        config.wl_screenrec.audio_device = "default".to_string();

        let command = build_wl_screenrec_command(&config, &WlScreenrecCapabilities::all()).unwrap();
        assert_eq!(command[0], "wl-screenrec");
        assert!(command.contains(&"--history".to_string()));
        assert!(command.contains(&"30".to_string()));
        assert!(command.contains(&"--max-fps".to_string()));
        assert!(command.contains(&"--ffmpeg-encoder".to_string()));
        assert!(command.contains(&"h264_nvenc".to_string()));
        assert!(command.contains(&"--audio".to_string()));
    }

    #[test]
    fn omits_optional_flags_when_help_does_not_support_them() {
        let mut config = Config::default();
        config.wl_screenrec.output = "DP-1".to_string();
        config.wl_screenrec.geometry = "0,0 1920x1080".to_string();
        let caps = WlScreenrecCapabilities::from_help("--history --filename");

        let command = build_wl_screenrec_command(&config, &caps).unwrap();
        assert!(command.contains(&"--history".to_string()));
        assert!(command.contains(&"--filename".to_string()));
        assert!(!command.contains(&"--ffmpeg-encoder".to_string()));
        assert!(!command.contains(&"--audio".to_string()));
        assert!(!command.contains(&"--output".to_string()));
        assert!(!command.contains(&"--geometry".to_string()));
    }

    #[test]
    fn output_mode_passes_configured_output() {
        let mut config = Config::default();
        config.wl_screenrec.output = "DP-1".to_string();
        config.wl_screenrec.auto_select_focused_output = false;

        let command = build_wl_screenrec_command(&config, &WlScreenrecCapabilities::all()).unwrap();
        assert!(command.contains(&"--output".to_string()));
        assert!(command.contains(&"DP-1".to_string()));
        assert!(!command.contains(&"--geometry".to_string()));
    }

    #[test]
    fn geometry_mode_passes_geometry_instead_of_output() {
        let mut config = Config::default();
        config.wl_screenrec.capture_mode = "geometry".to_string();
        config.wl_screenrec.output = "DP-1".to_string();
        config.wl_screenrec.geometry = "1366,0 1920x1080".to_string();

        let command = build_wl_screenrec_command(&config, &WlScreenrecCapabilities::all()).unwrap();
        assert!(command.contains(&"--geometry".to_string()));
        assert!(command.contains(&"1366,0 1920x1080".to_string()));
        assert!(!command.contains(&"--output".to_string()));
    }

    #[test]
    fn omits_audio_codec_when_audio_is_disabled() {
        let mut config = Config::default();
        config.wl_screenrec.audio = false;
        config.wl_screenrec.auto_select_focused_output = false;

        let command = build_wl_screenrec_command(&config, &WlScreenrecCapabilities::all()).unwrap();
        assert!(!command.contains(&"--audio".to_string()));
        assert!(!command.contains(&"--audio-codec".to_string()));
    }

    #[test]
    fn converts_ffmpeg_style_bitrate_for_wl_screenrec() {
        assert_eq!(bitrate_for_wl_screenrec("20M"), "2500 kB");
        assert_eq!(bitrate_for_wl_screenrec("800k"), "100000 B");
        assert_eq!(bitrate_for_wl_screenrec("5000000"), "5000000");
    }
}

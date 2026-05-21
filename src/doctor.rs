use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::Context;

use crate::config::Config;

pub const WLSCREENREC_REQUIRED_FLAGS: &[&str] = &["--history", "--filename"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum DoctorCheckStatus {
    Ok,
    Warn,
    Fail,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DoctorCheck {
    pub name: String,
    pub status: DoctorCheckStatus,
    pub message: String,
    pub hint: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DoctorReport {
    pub checks: Vec<DoctorCheck>,
    pub ok_count: usize,
    pub warn_count: usize,
    pub fail_count: usize,
    pub summary: String,
    pub recommended_actions: Vec<String>,
}

impl DoctorReport {
    pub fn from_checks(checks: Vec<DoctorCheck>) -> Self {
        let ok_count = checks
            .iter()
            .filter(|check| check.status == DoctorCheckStatus::Ok)
            .count();
        let warn_count = checks
            .iter()
            .filter(|check| check.status == DoctorCheckStatus::Warn)
            .count();
        let fail_count = checks
            .iter()
            .filter(|check| check.status == DoctorCheckStatus::Fail)
            .count();
        let recommended_actions = checks
            .iter()
            .filter_map(|check| check.hint.clone())
            .collect::<Vec<_>>();
        Self {
            checks,
            ok_count,
            warn_count,
            fail_count,
            summary: format!("{ok_count} OK, {warn_count} WARN, {fail_count} FAIL"),
            recommended_actions,
        }
    }
}

impl DoctorCheck {
    fn ok(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: DoctorCheckStatus::Ok,
            message: message.into(),
            hint: None,
        }
    }

    fn warn(name: impl Into<String>, message: impl Into<String>, hint: Option<String>) -> Self {
        Self {
            name: name.into(),
            status: DoctorCheckStatus::Warn,
            message: message.into(),
            hint,
        }
    }

    fn fail(name: impl Into<String>, message: impl Into<String>, hint: Option<String>) -> Self {
        Self {
            name: name.into(),
            status: DoctorCheckStatus::Fail,
            message: message.into(),
            hint,
        }
    }
}

pub fn run_report(config: &Config) -> anyhow::Result<DoctorReport> {
    let config_path = Config::config_file_path()?;
    Ok(DoctorReport::from_checks(collect_checks(
        config,
        &config_path,
    )))
}

fn collect_checks(config: &Config, config_path: &Path) -> Vec<DoctorCheck> {
    let mut checks = Vec::new();

    checks.push(DoctorCheck::ok(
        "config",
        format!("config loaded: {}", config_path.display()),
    ));

    checks.push(check_backend(config));
    checks.push(check_session());
    checks.push(check_shell());
    checks.push(check_v8q_on_path());
    checks.push(check_current_exe());
    checks.push(check_cargo_bin());
    checks.push(check_command("ffmpeg"));
    checks.push(check_command("ffprobe"));
    checks.push(check_command("wl-screenrec"));
    checks.push(check_command("pactl"));
    checks.push(check_command("notify-send"));
    checks.push(check_command("xdg-open"));
    checks.push(check_command("systemctl"));
    checks.push(check_command("hyprctl"));
    checks.push(check_wl_screenrec_flags(WLSCREENREC_REQUIRED_FLAGS));
    checks.push(check_wl_screenrec_optional_flags(&[
        "--max-fps",
        "--ffmpeg-encoder",
        "--audio",
        "--audio-device",
        "--audio-backend",
        "--audio-codec",
        "--bitrate",
        "--ffmpeg-encoder-options",
        "--output",
        "--geometry",
    ]));
    checks.push(check_ffmpeg_encoder(
        config
            .effective_encoder()
            .unwrap_or(&config.recording.encoder),
    ));
    checks.push(check_ffmpeg_encoder("h264_nvenc"));
    checks.push(check_ffmpeg_encoder("libx264"));
    checks.push(check_audio(config));
    checks.push(check_audio_default(config));
    checks.push(check_wl_screenrec_output_selection(config));
    checks.push(check_window_capture(config));
    checks.push(check_legacy_capture_window(config));
    checks.push(check_systemd_user_active("pipewire"));
    checks.push(check_systemd_user_active("wireplumber"));
    checks.push(check_systemd_user_active("xdg-desktop-portal"));
    checks.push(check_systemd_user_active("xdg-desktop-portal-hyprland"));
    checks.push(check_directory(
        "buffer directory",
        &config.paths.buffer_dir_path(),
    ));
    checks.push(check_legacy_buffer_dir(config));
    checks.push(check_directory(
        "output directory",
        &config.paths.output_dir_path(),
    ));
    checks.push(check_parent_directory(
        "PID file parent",
        &config.paths.pid_file_path(),
    ));

    checks
}

fn check_backend(config: &Config) -> DoctorCheck {
    match config.effective_backend() {
        Ok(backend) => DoctorCheck::ok(
            "backend",
            format!("backend recognized: {}", backend.as_str()),
        ),
        Err(error) => DoctorCheck::fail("backend", error.to_string(), None),
    }
}

fn check_session() -> DoctorCheck {
    let session = env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".to_string());
    let desktop = env::var("HYPRLAND_INSTANCE_SIGNATURE")
        .map(|_| "Hyprland".to_string())
        .or_else(|_| env::var("XDG_CURRENT_DESKTOP"))
        .unwrap_or_else(|_| "unknown".to_string());
    DoctorCheck::ok(
        "session",
        format!("session: {session}, compositor/desktop: {desktop}"),
    )
}

fn check_shell() -> DoctorCheck {
    let shell = env::var("SHELL").unwrap_or_else(|_| "unknown".to_string());
    DoctorCheck::ok("shell", format!("shell: {shell}"))
}

fn check_v8q_on_path() -> DoctorCheck {
    match command_path("v8q") {
        Some(path) => DoctorCheck::ok("v8q-path", format!("v8q found in PATH: {}", path.display())),
        None => DoctorCheck::warn(
            "v8q-path",
            "v8q is not in PATH",
            Some([
                "Run directly with: ~/.cargo/bin/v8q status".to_string(),
                "For bash: echo 'export PATH=\"$HOME/.cargo/bin:$PATH\"' >> ~/.bashrc && source ~/.bashrc".to_string(),
                "For zsh:  echo 'export PATH=\"$HOME/.cargo/bin:$PATH\"' >> ~/.zshrc && source ~/.zshrc".to_string(),
            ].join("\n")),
        ),
    }
}

fn check_current_exe() -> DoctorCheck {
    match env::current_exe() {
        Ok(path) => DoctorCheck::ok(
            "current-exe",
            format!("current executable: {}", path.display()),
        ),
        Err(error) => DoctorCheck::warn(
            "current-exe",
            "could not resolve current executable",
            Some(error.to_string()),
        ),
    }
}

fn check_cargo_bin() -> DoctorCheck {
    let Some(home) = dirs::home_dir() else {
        return DoctorCheck::warn("cargo-bin", "could not locate home directory", None);
    };
    let cargo_bin = home.join(".cargo/bin");

    if !cargo_bin.exists() {
        return DoctorCheck::warn(
            "cargo-bin",
            "~/.cargo/bin does not exist",
            Some("Run: cargo install --path .".to_string()),
        );
    }

    let path_has_cargo_bin = env::var_os("PATH")
        .and_then(|value| {
            env::split_paths(&value)
                .find(|path| path == &cargo_bin)
                .map(|_| ())
        })
        .is_some();

    if path_has_cargo_bin {
        DoctorCheck::ok(
            "cargo-bin",
            format!("~/.cargo/bin is in PATH: {}", cargo_bin.display()),
        )
    } else {
        DoctorCheck::warn(
            "cargo-bin",
            "~/.cargo/bin is not in PATH",
            Some(
                [
                    "Add this to ~/.bashrc:".to_string(),
                    "export PATH=\"$HOME/.cargo/bin:$PATH\"".to_string(),
                    "Then run: source ~/.bashrc".to_string(),
                ]
                .join("\n"),
            ),
        )
    }
}

fn check_command(command: &str) -> DoctorCheck {
    match command_path(command) {
        Some(path) => DoctorCheck::ok(command, format!("{command} found: {}", path.display())),
        None => DoctorCheck::fail(
            command,
            format!("{command} not found"),
            install_hint(command).map(Some).unwrap_or(None),
        ),
    }
}

fn check_wl_screenrec_flags(flags: &[&str]) -> DoctorCheck {
    let Ok(help) = wl_screenrec_help_text() else {
        return DoctorCheck::warn(
            "wl-screenrec-help",
            "could not inspect wl-screenrec --help",
            Some("Install wl-screenrec, then rerun: v8q doctor".to_string()),
        );
    };

    let missing: Vec<_> = flags
        .iter()
        .copied()
        .filter(|flag| !help.contains(flag))
        .collect();

    if missing.is_empty() {
        DoctorCheck::ok("wl-screenrec-flags", "wl-screenrec supports required flags")
    } else {
        DoctorCheck::fail(
            "wl-screenrec-flags",
            "wl-screenrec is missing flags used by V8Q",
            Some(
                missing
                    .into_iter()
                    .map(|flag| format!("missing {flag}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        )
    }
}

fn check_wl_screenrec_optional_flags(flags: &[&str]) -> DoctorCheck {
    let Ok(help) = wl_screenrec_help_text() else {
        return DoctorCheck::warn(
            "wl-screenrec-optional-flags",
            "could not inspect optional wl-screenrec flags",
            Some("Install wl-screenrec, then rerun: v8q doctor".to_string()),
        );
    };

    let missing: Vec<_> = flags
        .iter()
        .copied()
        .filter(|flag| !help.contains(flag))
        .collect();

    if missing.is_empty() {
        DoctorCheck::ok(
            "wl-screenrec-optional-flags",
            "optional wl-screenrec flags available",
        )
    } else {
        DoctorCheck::warn(
            "wl-screenrec-optional-flags",
            "optional wl-screenrec flags not available",
            Some(
                missing
                    .into_iter()
                    .map(|flag| format!("{flag} not found; V8Q does not pass it by default"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        )
    }
}

fn check_ffmpeg_encoder(encoder: &str) -> DoctorCheck {
    let Some(output) = command_stdout("ffmpeg", &["-hide_banner", "-encoders"]) else {
        return DoctorCheck::fail(
            "ffmpeg-encoder",
            "could not inspect ffmpeg encoders",
            Some("Install ffmpeg: sudo pacman -S ffmpeg".to_string()),
        );
    };

    if output.contains(encoder) {
        DoctorCheck::ok(
            "ffmpeg-encoder",
            format!("ffmpeg encoder available: {encoder}"),
        )
    } else {
        DoctorCheck::fail(
            "ffmpeg-encoder",
            format!("ffmpeg encoder not found: {encoder}"),
            Some(
                [
                    "For NVIDIA NVENC, confirm your NVIDIA driver and FFmpeg build support NVENC."
                        .to_string(),
                    "Try: ffmpeg -hide_banner -encoders | grep nvenc".to_string(),
                ]
                .join("\n"),
            ),
        )
    }
}

fn check_audio(config: &Config) -> DoctorCheck {
    let Some(output) = command_stdout("pactl", &["list", "short", "sources"]) else {
        return DoctorCheck::warn(
            "audio",
            "could not inspect pactl sources",
            Some("Install/check pipewire-pulse and pactl".to_string()),
        );
    };
    let monitor_count = output
        .lines()
        .filter(|line| line.contains(".monitor"))
        .count();
    if config.wl_screenrec.audio && monitor_count == 0 {
        return DoctorCheck::warn(
            "audio",
            "audio enabled but no .monitor source found",
            Some("Run: pactl list short sources".to_string()),
        );
    }
    if config.wl_screenrec.audio
        && config.wl_screenrec.audio_device != "default"
        && !output.contains(&config.wl_screenrec.audio_device)
    {
        return DoctorCheck::warn(
            "audio",
            "configured audio_device was not found",
            Some(config.wl_screenrec.audio_device.clone()),
        );
    }
    DoctorCheck::ok(
        "audio",
        format!("pactl sources available; monitor sources: {monitor_count}"),
    )
}

fn check_audio_default(config: &Config) -> DoctorCheck {
    if config.wl_screenrec.audio && config.wl_screenrec.audio_device == "default" {
        DoctorCheck::warn(
            "audio-device",
            "audio_device is 'default'; wl-screenrec may fail on some PipeWire setups",
            Some(
                "Prefer a .monitor source from `v8q audio sources`, or set `[wl_screenrec] audio = false` for testing."
                    .to_string(),
            ),
        )
    } else {
        DoctorCheck::ok(
            "audio-device",
            "wl-screenrec audio_device is explicit or audio is off",
        )
    }
}

fn check_legacy_buffer_dir(config: &Config) -> DoctorCheck {
    let buffer_dir = config.paths.buffer_dir_path();
    if buffer_dir == Path::new("/tmp/v8q-buffer") {
        DoctorCheck::warn(
            "buffer-directory",
            "Using legacy buffer_dir /tmp/v8q-buffer",
            Some(format!(
                "Recommended runtime buffer: {}. Run `v8q config migrate --write` when you are ready to migrate paths.",
                crate::paths::default_buffer_dir().display()
            )),
        )
    } else {
        DoctorCheck::ok(
            "buffer-directory",
            format!(
                "buffer_dir is not using the legacy default: {}",
                buffer_dir.display()
            ),
        )
    }
}

fn check_wl_screenrec_output_selection(config: &Config) -> DoctorCheck {
    let Ok(backend) = config.effective_backend() else {
        return DoctorCheck::warn("wl-screenrec-output", "backend is invalid", None);
    };
    if backend != crate::CaptureBackend::WlScreenrec {
        return DoctorCheck::ok(
            "wl-screenrec-output",
            "wl-screenrec output selection not needed",
        );
    }
    if matches!(
        config.wl_screenrec.capture_mode.as_str(),
        "active-window" | "window"
    ) {
        return DoctorCheck::ok(
            "wl-screenrec-output",
            "wl-screenrec is configured to capture the active window with --geometry",
        );
    }
    if !config.wl_screenrec.output.trim().is_empty()
        || !config.wl_screenrec.geometry.trim().is_empty()
    {
        return DoctorCheck::ok(
            "wl-screenrec-output",
            "wl-screenrec output/geometry is configured",
        );
    }
    if config.wl_screenrec.auto_select_focused_output {
        return DoctorCheck::ok(
            "wl-screenrec-output",
            "wl-screenrec will auto-select the focused Hyprland output",
        );
    }
    let Some(monitors) = command_stdout("hyprctl", &["monitors", "-j"]) else {
        return DoctorCheck::warn(
            "wl-screenrec-output",
            "could not inspect Hyprland monitors",
            Some("If wl-screenrec reports multiple displays, set `[wl_screenrec] output = \"DP-1\"` or another output from `hyprctl monitors -j`.".to_string()),
        );
    };
    let names = monitor_names_from_hyprctl_json(&monitors);
    if names.len() > 1 {
        DoctorCheck::warn(
            "wl-screenrec-output",
            "multiple enabled displays detected and wl_screenrec.output is empty",
            Some(format!(
                "wl-screenrec may exit before recording. Set `[wl_screenrec] output = \"{}\"` or another output from: {}",
                names[0],
                names.join(", ")
            )),
        )
    } else {
        DoctorCheck::ok(
            "wl-screenrec-output",
            "single display detected or no explicit output needed",
        )
    }
}

fn check_window_capture(config: &Config) -> DoctorCheck {
    let help = wl_screenrec_help_text().unwrap_or_default();
    let capture_window = config.effective_capture_window();
    if !capture_window.enabled {
        return DoctorCheck::ok("window-capture", "window capture is disabled");
    }
    if !help.contains("--geometry") {
        return DoctorCheck::fail(
            "window-capture",
            "window capture requires wl-screenrec --geometry",
            Some("Update wl-screenrec or use fullscreen capture.".to_string()),
        );
    }
    if capture_window.geometry.trim().is_empty() {
        return DoctorCheck::fail(
            "window-capture",
            "window capture is enabled but no geometry is configured",
            Some("Run: v8q window select".to_string()),
        );
    }
    if capture_window.follow {
        match crate::window::selected_window_geometry(config) {
            Ok(Some(_)) => DoctorCheck::ok("window-capture", "selected window can be resolved"),
            Ok(None) => DoctorCheck::warn(
                "window-capture",
                "window capture is enabled but no window was resolved",
                Some("Run: v8q window select".to_string()),
            ),
            Err(error) => DoctorCheck::fail(
                "window-capture",
                "selected window could not be resolved",
                Some(error.to_string()),
            ),
        }
    } else {
        DoctorCheck::ok(
            "window-capture",
            format!(
                "window capture uses fixed geometry: {}",
                capture_window.geometry
            ),
        )
    }
}

fn check_legacy_capture_window(config: &Config) -> DoctorCheck {
    if config.capture_window.is_some() {
        DoctorCheck::warn(
            "legacy-capture-window",
            "legacy [capture_window] is present; [capture.window] takes precedence",
            Some("Run: v8q config migrate --write".to_string()),
        )
    } else {
        DoctorCheck::ok(
            "legacy-capture-window",
            "no legacy [capture_window] section",
        )
    }
}

fn monitor_names_from_hyprctl_json(text: &str) -> Vec<String> {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|monitor| {
            monitor
                .get("name")
                .and_then(|name| name.as_str())
                .map(ToString::to_string)
        })
        .collect()
}

fn check_systemd_user_active(service: &str) -> DoctorCheck {
    let output = Command::new("systemctl")
        .arg("--user")
        .arg("is-active")
        .arg(service)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match output {
        Ok(output) if output.status.success() => {
            DoctorCheck::ok(service, format!("{service} active"))
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let state = if stdout.is_empty() { stderr } else { stdout };
            DoctorCheck::fail(
                service,
                format!("{service} inactive"),
                Some(format!("systemctl reported: {state}")),
            )
        }
        Err(error) => DoctorCheck::warn(
            service,
            format!("could not check {service}"),
            Some(error.to_string()),
        ),
    }
}

fn check_directory(label: &str, path: &Path) -> DoctorCheck {
    match std::fs::create_dir_all(path) {
        Ok(()) => DoctorCheck::ok(
            label,
            format!("{label} exists or can be created: {}", path.display()),
        ),
        Err(error) => DoctorCheck::fail(
            label,
            format!("{label} cannot be created: {}", path.display()),
            Some(error.to_string()),
        ),
    }
}

fn check_parent_directory(label: &str, path: &Path) -> DoctorCheck {
    let Some(parent) = path.parent() else {
        return DoctorCheck::fail(
            label,
            format!("{label} has no parent: {}", path.display()),
            None,
        );
    };

    check_directory(label, parent)
}

pub fn command_path(command: &str) -> Option<PathBuf> {
    if command.contains('/') {
        let path = PathBuf::from(command);
        return path.is_file().then_some(path);
    }

    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths)
            .map(|path| path.join(command))
            .find(|candidate| candidate.is_file())
    })
}

pub fn command_stdout(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    Some(text)
}

pub fn wl_screenrec_help_text() -> anyhow::Result<String> {
    command_stdout("wl-screenrec", &["--help"]).context("failed to run wl-screenrec --help")
}

pub fn wl_screenrec_help_has_flags(help: &str, flags: &[&str]) -> anyhow::Result<()> {
    for flag in flags {
        if !help.contains(flag) {
            anyhow::bail!("wl-screenrec --help does not contain required flag {flag}");
        }
    }

    Ok(())
}

fn install_hint(command: &str) -> Option<String> {
    match command {
        "ffmpeg" => Some("Install on Arch/Omarchy: sudo pacman -S ffmpeg".to_string()),
        "wl-screenrec" => Some("Install on Arch/Omarchy: paru -S wl-screenrec".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{DoctorCheck, DoctorCheckStatus, DoctorReport};

    #[test]
    fn command_helpers_do_not_panic_for_missing_commands() {
        assert!(super::command_path("definitely-not-a-v8q-command").is_none());
        assert!(super::command_stdout("definitely-not-a-v8q-command", &["--help"]).is_none());
    }

    #[test]
    fn report_counts_statuses() {
        let report = DoctorReport::from_checks(vec![
            DoctorCheck::ok("a", "ok"),
            DoctorCheck::warn("b", "warn", None),
            DoctorCheck::fail("c", "fail", None),
            DoctorCheck {
                name: "d".to_string(),
                status: DoctorCheckStatus::Ok,
                message: "ok2".to_string(),
                hint: None,
            },
        ]);

        assert_eq!(report.ok_count, 2);
        assert_eq!(report.warn_count, 1);
        assert_eq!(report.fail_count, 1);
    }

    #[test]
    fn audio_default_generates_warning() {
        let mut config = crate::Config::default();
        config.wl_screenrec.audio = true;
        config.wl_screenrec.audio_device = "default".to_string();

        let check = super::check_audio_default(&config);
        assert_eq!(check.status, DoctorCheckStatus::Warn);
        assert!(check.message.contains("audio_device is 'default'"));
    }

    #[test]
    fn legacy_buffer_generates_warning() {
        let mut config = crate::Config::default();
        config.paths.buffer_dir = "/tmp/v8q-buffer".to_string();

        let check = super::check_legacy_buffer_dir(&config);
        assert_eq!(check.status, DoctorCheckStatus::Warn);
        assert!(check.message.contains("legacy buffer_dir"));
        assert_ne!(
            crate::paths::default_buffer_dir(),
            PathBuf::from("/tmp/v8q-buffer")
        );
    }

    #[test]
    fn parses_hyprctl_monitor_names() {
        let names = super::monitor_names_from_hyprctl_json(
            r#"[
  {"name": "HDMI-A-1"},
  {"name": "DP-1"}
]"#,
        );
        assert_eq!(names, vec!["HDMI-A-1", "DP-1"]);
    }
}

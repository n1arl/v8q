use std::fs::OpenOptions;
use std::io::{IsTerminal, Read, Write};
use std::process::{Command as ProcessCommand, Stdio};

use anyhow::Context;
use clap::Parser;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use v8q::cli::{
    AudioCommand, CleanCommand, Cli, ClipCommand, ClipSort, Command, ConfigCommand, DebugCommand,
    ModeCommand, PresetCommand, ServiceCommand, SetupCommand, WindowCommand,
};
use v8q::{DoctorCheckStatus, Result};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config =
        v8q::Config::load_or_create_default().context("failed to load or create V8Q config")?;

    match cli.command {
        Command::Welcome => print_welcome(),
        Command::Start {
            foreground,
            target,
            output,
            geometry,
        } => {
            let mut start_config = config.clone();
            if let Some(target) = target {
                start_config.wl_screenrec.capture_mode = target;
            }
            if let Some(output) = output {
                start_config.wl_screenrec.output = output;
            }
            if let Some(geometry) = geometry {
                start_config.wl_screenrec.geometry = geometry;
                start_config.wl_screenrec.capture_mode = "geometry".to_string();
            }
            let result = v8q::start_recorder(&start_config)?;
            print_start(result.clone());
            if foreground {
                run_foreground(&start_config)?;
            }
        }
        Command::Stop => print_stop(v8q::stop_recorder(&config)?),
        Command::Status { json } => {
            let status = v8q::get_status(&config)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&status)?);
            } else {
                print_status(status);
            }
        }
        Command::Doctor {
            json,
            verbose,
            fix_plan,
        } => {
            let report = v8q::run_doctor(&config)?;
            if fix_plan {
                print_fix_plan(&report, &config);
            } else if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else if verbose || config.ui.mode == "advanced" {
                print_doctor(&report);
            } else {
                print_doctor_summary(&report);
            }
            if report.fail_count > 0 && !fix_plan {
                std::process::exit(1);
            }
        }
        Command::Save {
            name,
            duration,
            open,
            reveal,
            no_notify,
            json,
        } => {
            let result = v8q::save_replay_with_options(
                &config,
                &v8q::replay::SaveOptions {
                    name,
                    duration_seconds: duration,
                },
            )?;
            if !no_notify {
                v8q::notify_replay_saved(&config, &result.output_file);
            }
            if open {
                v8q::open_path(&result.output_file)?;
            }
            if reveal {
                reveal_path(&result.output_file)?;
            }
            if json {
                let size_bytes = std::fs::metadata(&result.output_file)
                    .map(|metadata| metadata.len())
                    .unwrap_or(0);
                let payload = v8q::SaveJson {
                    success: true,
                    output_path: result.output_file,
                    backend: result.backend,
                    duration_seconds: result.duration_seconds,
                    size_bytes,
                    saved_at: chrono::Local::now().to_rfc3339(),
                    warnings: Vec::new(),
                };
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("Saved replay: {}", result.output_file.display());
            }
        }
        Command::Clean { command } => handle_clean(command, &config)?,
        Command::OpenFolder => {
            v8q::open_path(config.paths.output_dir_path())?;
        }
        Command::Logs {
            follow,
            backend,
            lines,
            clear,
        } => handle_logs(&config, follow, backend, lines, clear)?,
        Command::Config { command } => handle_config(command, &config)?,
        Command::Clips {
            latest,
            open_latest,
            delete_latest,
            json,
            limit,
            sort,
        } => handle_clips(
            &config,
            latest,
            open_latest,
            delete_latest,
            json,
            limit,
            sort,
        )?,
        Command::Clip { command } => handle_clip(command, &config)?,
        Command::Setup { command } => handle_setup(command, &config)?,
        Command::Preset { command } => handle_preset(command, &config)?,
        Command::Mode { command } => handle_mode(command, &config)?,
        Command::Windows { json } => handle_windows(json)?,
        Command::Window { command } => handle_window(command, &config)?,
        Command::Service { command } => handle_service(command)?,
        Command::Debug { command } => handle_debug(command, &config)?,
        Command::Audio { command } => handle_audio(command)?,
    }

    Ok(())
}

fn print_welcome() {
    println!("V8Q - Linux Replay Recorder\n");
    println!("V8Q keeps a small replay buffer and saves recent clips on demand.");
    println!("Start here:\n");
    println!("  v8q doctor");
    println!("  v8q preset apply beginner-safe --write");
    println!("  v8q start    # then: v8q save, v8q stop");
    if !cargo_bin_in_path() {
        println!("\nWARN: ~/.cargo/bin is not in PATH.");
        println!("Add it with: export PATH=\"$HOME/.cargo/bin:$PATH\"");
        println!("Or run directly: ~/.cargo/bin/v8q doctor");
    }
}

fn print_start(result: v8q::StartResult) {
    println!("Started V8Q recorder with PID {}.", result.pid);
    println!("Backend: {}", result.backend);
    println!("Buffer: {}", result.buffer_dir.display());
    println!("Log: {}", result.log_file.display());
}

fn print_stop(result: v8q::StopResult) {
    match (result.was_running, result.pid) {
        (true, Some(pid)) => println!("Stopped V8Q recorder with PID {pid}."),
        (false, Some(pid)) => {
            println!("Recorder PID {pid} was not running; removed stale PID metadata.")
        }
        (false, None) => println!("V8Q is not running."),
        (true, None) => println!("Stopped V8Q recorder."),
    }
}

fn print_status(status: v8q::StatusInfo) {
    println!("V8Q Status\n");
    println!(
        "Recorder: {}",
        if status.is_running {
            "running"
        } else {
            "stopped"
        }
    );
    if let Some(pid) = status.pid {
        println!("PID: {pid}");
    }
    println!("Backend: {}", status.backend);
    if let Some(preset) = &status.detected_preset {
        println!("Preset: {preset}");
    }
    println!("Capture target: {}", status.capture_target);
    println!("Config: {}", status.config_path.display());
    println!("Buffer: {}", status.buffer_dir.display());
    println!("Output: {}", status.output_dir.display());
    if let Some(path) = &status.latest_clip {
        println!("Latest clip: {}", path.display());
    }
    if let Some(path) = &status.log_file {
        println!("Log file: {}", path.display());
    }
    println!("Replay: {}s", status.replay_duration);
    println!("Segment: {}s", status.segment_duration);
    println!("FPS: {}", status.fps);
    println!("Encoder: {}", status.encoder);
    println!("Bitrate: {}", status.bitrate);
    if let Some(audio) = status.audio_enabled {
        println!("Audio: {}", if audio { "on" } else { "off" });
    }
    if let Some(count) = status.segment_count {
        println!("Segments: {count}");
    }
    let has_history_backend = status.history_file.is_some();
    if let Some(path) = status.history_file {
        println!("History file: {}", path.display());
    }
    if let Some(exists) = status.history_exists {
        println!("History exists: {exists}");
    }
    if let Some(size) = status.history_size_bytes {
        println!("History size: {size} bytes");
    }
    if has_history_backend {
        let label = match status.history_valid {
            Some(true) => "yes",
            Some(false) => "no",
            None => "unknown",
        };
        println!("History valid: {label}");
        if status.history_valid == Some(false) {
            println!(
                "History warning: History file exists but is probably not a valid replay yet."
            );
        }
    }
    if let Some(command) = status.process_command {
        println!("Process command: {command}");
    }
    if let Some(meta) = status.metadata {
        println!("Started at: {}", meta.started_at);
        println!("Recorded command: {}", meta.command);
    }
    for warning in status.warnings {
        println!("WARN: {warning}");
    }
    if let Some(error) = status.error {
        println!("ERROR: {error}");
    }
    if !status.log_tail.is_empty() {
        println!("\nLast wl-screenrec log:");
        for line in status.log_tail {
            println!("  {line}");
        }
    }
    if !status.last_error_lines.is_empty() {
        println!("\nRecent relevant log lines:");
        for line in status.last_error_lines {
            println!("  {line}");
        }
    }
    print_logs(v8q::LogsResult {
        logs: status.last_log_lines,
    });
}

fn print_doctor(report: &v8q::DoctorReport) {
    println!("V8Q Doctor\n");
    for check in &report.checks {
        let label = match check.status {
            DoctorCheckStatus::Ok => "OK",
            DoctorCheckStatus::Warn => "WARN",
            DoctorCheckStatus::Fail => "FAIL",
        };
        println!("[{label}] {}", check.message);
        if let Some(hint) = &check.hint {
            for line in hint.lines() {
                println!("       {line}");
            }
        }
    }
    println!("\nSummary:");
    println!("- {} OK", report.ok_count);
    println!("- {} WARN", report.warn_count);
    println!("- {} FAIL", report.fail_count);
}

fn print_doctor_summary(report: &v8q::DoctorReport) {
    println!("V8Q Doctor\n");
    let groups = [
        (
            "System",
            ["session", "shell", "v8q-path", "current-exe", "cargo-bin"].as_slice(),
        ),
        (
            "Backend",
            [
                "backend",
                "wl-screenrec",
                "wl-screenrec-flags",
                "wl-screenrec-output",
            ]
            .as_slice(),
        ),
        ("Audio", ["audio", "audio-device", "pactl"].as_slice()),
        (
            "Paths",
            [
                "buffer directory",
                "buffer-directory",
                "output directory",
                "PID file parent",
            ]
            .as_slice(),
        ),
        ("Window capture", ["hyprctl", "window-capture"].as_slice()),
    ];
    for (label, names) in groups {
        let status = report
            .checks
            .iter()
            .filter(|check| names.iter().any(|name| check.name.contains(name)))
            .map(|check| check.status)
            .max_by_key(|status| match status {
                DoctorCheckStatus::Ok => 0,
                DoctorCheckStatus::Warn => 1,
                DoctorCheckStatus::Fail => 2,
            })
            .unwrap_or(DoctorCheckStatus::Ok);
        let text = match status {
            DoctorCheckStatus::Ok => "OK",
            DoctorCheckStatus::Warn => "WARN",
            DoctorCheckStatus::Fail => "FAIL",
        };
        println!("{label}: {text}");
    }
    println!("\nSummary: {}", report.summary);
    if !report.recommended_actions.is_empty() {
        println!("\nRecommended actions:");
        for action in report.recommended_actions.iter().take(6) {
            println!("- {}", action.lines().next().unwrap_or(action));
        }
    }
    println!("\nRun `v8q doctor --verbose` for details.");
}

fn print_fix_plan(report: &v8q::DoctorReport, config: &v8q::Config) {
    println!("V8Q Fix Plan\n");
    println!("No changes were made.\n");
    if !cargo_bin_in_path() {
        println!("- Add Cargo bin to PATH:");
        println!("  echo 'export PATH=\"$HOME/.cargo/bin:$PATH\"' >> ~/.bashrc");
        println!("  source ~/.bashrc");
    }
    if report
        .checks
        .iter()
        .any(|check| check.name == "ffmpeg" && check.status != DoctorCheckStatus::Ok)
    {
        println!("- Install FFmpeg:");
        println!("  sudo pacman -S ffmpeg");
    }
    if report
        .checks
        .iter()
        .any(|check| check.name == "wl-screenrec" && check.status != DoctorCheckStatus::Ok)
    {
        println!("- Install wl-screenrec:");
        println!("  paru -S wl-screenrec");
    }
    if report.checks.iter().any(|check| {
        check.name == "xdg-desktop-portal-hyprland" && check.status != DoctorCheckStatus::Ok
    }) {
        println!("- Install/enable Hyprland portal:");
        println!("  sudo pacman -S xdg-desktop-portal xdg-desktop-portal-hyprland");
        println!("  systemctl --user restart xdg-desktop-portal xdg-desktop-portal-hyprland");
    }
    if v8q::preset::detect(config).is_none() {
        println!("- Apply the safest first-run preset:");
        println!("  v8q preset apply beginner-safe --write");
    }
    println!("- Test capture:");
    println!("  v8q debug wl-screenrec --test-run 5");
    println!("- Smoke test:");
    println!("  v8q start && sleep 10 && v8q save --name smoke && v8q stop");
}

fn print_logs(result: v8q::LogsResult) {
    for (path, lines) in result.logs {
        println!("\nLast log lines from {}:", path.display());
        for line in lines {
            println!("  {line}");
        }
    }
}

fn handle_config(command: ConfigCommand, config: &v8q::Config) -> Result<()> {
    match command {
        ConfigCommand::Path => println!("{}", v8q::config_path()?.display()),
        ConfigCommand::Show { resolved: _, json } => {
            if json {
                println!("{}", serde_json::to_string_pretty(config)?);
            } else {
                println!("{}", toml::to_string_pretty(config)?);
            }
        }
        ConfigCommand::Validate => {
            let warnings = v8q::validate_config_detailed(config)?;
            println!("Config OK");
            for warning in warnings {
                println!("WARN: {warning}");
            }
        }
        ConfigCommand::Edit => {
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
            let status = ProcessCommand::new(editor)
                .arg(v8q::config_path()?)
                .status()
                .context("failed to open editor")?;
            if !status.success() {
                anyhow::bail!("editor exited with non-zero status");
            }
        }
        ConfigCommand::Init { force } => {
            let path = v8q::config_path()?;
            if path.exists() && !force {
                anyhow::bail!(
                    "config already exists at {}; use --force to overwrite",
                    path.display()
                );
            }
            v8q::save_config(&v8q::Config::default())?;
            println!("Wrote config: {}", path.display());
        }
        ConfigCommand::Reset => {
            let backup = v8q::migrate_config().ok();
            v8q::save_config(&v8q::Config::default())?;
            if let Some(backup) = backup {
                println!("Backup: {}", backup.display());
            }
            println!("Reset config: {}", v8q::config_path()?.display());
        }
        ConfigCommand::Migrate { write } => {
            if write {
                let backup = v8q::migrate_config()?;
                println!("Migrated config. Backup: {}", backup.display());
            } else {
                println!("Dry run only. Re-run with `v8q config migrate --write` to update:");
                println!("- add [capture] backend if only legacy [ffmpeg].capture_backend exists");
                println!("- keep a timestamped config.toml.bak-* backup");
                println!(
                    "- recommended buffer_dir: {}",
                    v8q::paths::default_buffer_dir().display()
                );
            }
        }
    }
    Ok(())
}

fn handle_clean(command: Option<CleanCommand>, config: &v8q::Config) -> Result<()> {
    match command {
        None => {
            let result = v8q::clean_buffer(config)?;
            println!(
                "Removed {} file(s) from {}",
                result.removed_files,
                result.buffer_dir.display()
            );
        }
        Some(CleanCommand::Logs { older_than }) => {
            let removed = clean_logs(older_than.as_deref())?;
            println!("Removed {removed} log file(s).");
        }
    }
    Ok(())
}

fn handle_logs(
    _config: &v8q::Config,
    follow: bool,
    backend: Option<String>,
    lines: usize,
    clear: bool,
) -> Result<()> {
    let files = log_files(backend.as_deref());
    if clear {
        let mut removed = 0usize;
        for file in &files {
            if file.exists() {
                std::fs::remove_file(file)?;
                removed += 1;
            }
        }
        println!("Removed {removed} log file(s).");
        return Ok(());
    }
    if follow {
        let mut command = ProcessCommand::new("tail");
        command.arg("-f");
        for file in files {
            command.arg(file);
        }
        let status = command.status().context("failed to run tail -f")?;
        if !status.success() {
            anyhow::bail!("tail -f failed");
        }
        return Ok(());
    }
    for file in files {
        if file.exists() {
            let output = ProcessCommand::new("tail")
                .arg("-n")
                .arg(lines.to_string())
                .arg(&file)
                .output()
                .with_context(|| format!("failed to read {}", file.display()))?;
            println!("\n{}:", file.display());
            print!("{}", String::from_utf8_lossy(&output.stdout));
        }
    }
    Ok(())
}

fn handle_clips(
    config: &v8q::Config,
    latest: bool,
    open_latest: bool,
    delete_latest: bool,
    json: bool,
    limit: Option<usize>,
    sort: ClipSort,
) -> Result<()> {
    if latest || open_latest {
        let Some(path) = v8q::latest_clip(config)? else {
            println!("No clips found.");
            return Ok(());
        };
        if open_latest {
            v8q::open_path(&path)?;
        } else {
            println!("{}", path.display());
        }
        return Ok(());
    }
    if delete_latest {
        let Some(path) = v8q::latest_clip(config)? else {
            println!("No clips found.");
            return Ok(());
        };
        delete_clip(config, &path, false)?;
        println!("Deleted {}", path.display());
        return Ok(());
    }

    let mut clips = v8q::list_clips(config)?.clips;
    if matches!(sort, ClipSort::Newest) {
        clips.reverse();
    }
    if let Some(limit) = limit {
        clips.truncate(limit);
    }
    if json {
        let rows = clips
            .iter()
            .map(|path| clip_summary(config, path))
            .collect::<Result<Vec<_>>>()?;
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else {
        for clip in clips {
            let row = clip_summary(config, &clip)?;
            println!(
                "{} | {} bytes | {} | {}",
                row.name,
                row.size_bytes,
                row.duration
                    .map(|duration| format!("{duration:.3}s"))
                    .unwrap_or_else(|| "duration unknown".to_string()),
                row.path.display()
            );
        }
    }
    Ok(())
}

fn handle_clip(command: ClipCommand, config: &v8q::Config) -> Result<()> {
    match command {
        ClipCommand::Info { path } => {
            let path = PathBuf::from(path);
            ensure_clip_in_output(config, &path, true)?;
            let metadata = std::fs::metadata(&path)?;
            println!("Path: {}", path.display());
            println!("Size: {} bytes", metadata.len());
            if let Ok(modified) = metadata.modified() {
                println!("Modified: {:?}", modified);
            }
            match ffprobe_clip(&path) {
                Ok(Some(info)) => {
                    println!("Duration: {:.3}s", info.duration.unwrap_or(0.0));
                    println!(
                        "Codec: {}",
                        info.codec.unwrap_or_else(|| "unknown".to_string())
                    );
                    if let (Some(width), Some(height)) = (info.width, info.height) {
                        println!("Resolution: {width}x{height}");
                    }
                    if let Some(level) = info.level {
                        println!("Level: {level}");
                    }
                    if let Some(has_b_frames) = info.has_b_frames {
                        println!("Has B-frames: {has_b_frames}");
                    }
                }
                Ok(None) => {
                    println!("ffprobe unavailable; install ffmpeg to inspect codec/duration.");
                }
                Err(error) => println!("ffprobe failed: {error:#}"),
            }
        }
        ClipCommand::Open { path } => v8q::open_path(path)?,
        ClipCommand::Reveal { path } => reveal_path(Path::new(&path))?,
        ClipCommand::Delete {
            path,
            force_external,
        } => {
            let path = PathBuf::from(path);
            delete_clip(config, &path, force_external)?;
            println!("Deleted {}", path.display());
        }
        ClipCommand::Rename {
            path,
            new_name,
            overwrite,
        } => {
            let path = PathBuf::from(path);
            ensure_clip_in_output(config, &path, false)?;
            let name = v8q::sanitize_clip_name(&new_name);
            let target = config.paths.output_dir_path().join(format!("{name}.mkv"));
            if target.exists() && !overwrite {
                anyhow::bail!("target exists: {}; use --overwrite", target.display());
            }
            std::fs::rename(&path, &target)?;
            println!("{}", target.display());
        }
    }
    Ok(())
}

#[derive(Debug, serde::Serialize)]
struct ClipSummary {
    name: String,
    path: PathBuf,
    size_bytes: u64,
    duration: Option<f64>,
    modified: Option<String>,
}

#[derive(Debug)]
struct ProbeInfo {
    duration: Option<f64>,
    codec: Option<String>,
    width: Option<u64>,
    height: Option<u64>,
    level: Option<u64>,
    has_b_frames: Option<u64>,
}

fn clip_summary(_config: &v8q::Config, path: &Path) -> Result<ClipSummary> {
    let metadata = std::fs::metadata(path)?;
    let duration = ffprobe_clip(path)
        .ok()
        .flatten()
        .and_then(|info| info.duration);
    let modified = metadata
        .modified()
        .ok()
        .map(|time| chrono::DateTime::<chrono::Local>::from(time).to_rfc3339());
    Ok(ClipSummary {
        name: path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("<unknown>")
            .to_string(),
        path: path.to_path_buf(),
        size_bytes: metadata.len(),
        duration,
        modified,
    })
}

fn ffprobe_clip(path: &Path) -> Result<Option<ProbeInfo>> {
    if v8q::doctor::command_path("ffprobe").is_none() {
        return Ok(None);
    }
    let output = ProcessCommand::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration:stream=codec_name,width,height,level,has_b_frames",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .context("failed to run ffprobe")?;
    if !output.status.success() {
        anyhow::bail!("{}", String::from_utf8_lossy(&output.stderr));
    }
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let stream = json
        .get("streams")
        .and_then(|streams| streams.as_array())
        .and_then(|streams| streams.first());
    Ok(Some(ProbeInfo {
        duration: json
            .get("format")
            .and_then(|format| format.get("duration"))
            .and_then(|duration| duration.as_str())
            .and_then(|duration| duration.parse::<f64>().ok()),
        codec: stream
            .and_then(|stream| stream.get("codec_name"))
            .and_then(|codec| codec.as_str())
            .map(ToString::to_string),
        width: stream
            .and_then(|stream| stream.get("width"))
            .and_then(|width| width.as_u64()),
        height: stream
            .and_then(|stream| stream.get("height"))
            .and_then(|height| height.as_u64()),
        level: stream
            .and_then(|stream| stream.get("level"))
            .and_then(|level| level.as_u64()),
        has_b_frames: stream
            .and_then(|stream| stream.get("has_b_frames"))
            .and_then(|frames| frames.as_u64()),
    }))
}

fn reveal_path(path: &Path) -> Result<()> {
    let dir = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| anyhow::anyhow!("path has no parent: {}", path.display()))?
    };
    v8q::open_path(dir)
}

fn handle_setup(command: Option<SetupCommand>, config: &v8q::Config) -> Result<()> {
    std::fs::create_dir_all(config.paths.output_dir_path())?;
    std::fs::create_dir_all(config.paths.buffer_dir_path())?;
    v8q::validate_config(config)?;
    let report = v8q::run_doctor(config)?;
    match command {
        Some(SetupCommand::Shell { write }) => print_shell_setup(write)?,
        Some(SetupCommand::Hyprland { write }) => print_hyprland_setup(write)?,
        None => {
            println!("V8Q setup complete.\n");
            print_doctor_summary(&report);
            if std::io::stdin().is_terminal() {
                interactive_setup(config)?;
            }
            println!("\nBasic commands:");
            println!("  v8q start");
            println!("  v8q save");
            println!("  v8q stop");
            println!("  v8q clips");
            println!("  v8q doctor");
            println!();
            print_shell_setup(false)?;
            print_hyprland_setup(false)?;
        }
    }
    Ok(())
}

fn interactive_setup(config: &v8q::Config) -> Result<()> {
    println!("\nInteractive setup:");
    if prompt_yes_no("Use beginner mode?", config.ui.mode != "advanced")? {
        let mut next = config.clone();
        next.ui.mode = "beginner".to_string();
        let backup = backup_config_file()?;
        v8q::save_config(&next)?;
        println!("Mode set to beginner. Backup: {}", backup.display());
    }
    if prompt_yes_no("Apply beginner-safe preset?", false)? {
        let preset = v8q::preset::find("beginner-safe")
            .ok_or_else(|| anyhow::anyhow!("missing beginner-safe preset"))?;
        let mut next = v8q::Config::load_or_create_default()?;
        let backup = backup_config_file()?;
        v8q::preset::apply(&mut next, &preset);
        v8q::save_config(&next)?;
        println!("Applied beginner-safe. Backup: {}", backup.display());
    }
    if prompt_yes_no("Run wl-screenrec test now?", false)? {
        let latest = v8q::Config::load_or_create_default()?;
        debug_wl_screenrec_test_run(&latest, 5)?;
    }
    if prompt_yes_no("Show suggested Hyprland binds?", true)? {
        print_hyprland_setup(false)?;
    }
    Ok(())
}

fn prompt_yes_no(question: &str, default_yes: bool) -> Result<bool> {
    let suffix = if default_yes { "[Y/n]" } else { "[y/N]" };
    print!("{question} {suffix} ");
    std::io::stdout().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let value = input.trim().to_ascii_lowercase();
    Ok(if value.is_empty() {
        default_yes
    } else {
        matches!(value.as_str(), "y" | "yes" | "s" | "sim")
    })
}

fn handle_preset(command: PresetCommand, config: &v8q::Config) -> Result<()> {
    match command {
        PresetCommand::List => {
            for preset in v8q::preset::presets() {
                println!("{} - {}", preset.name, preset.description);
            }
        }
        PresetCommand::Explain { name } => {
            let preset = v8q::preset::find(&name)
                .ok_or_else(|| anyhow::anyhow!("unknown preset: {name}"))?;
            println!("{}", v8q::preset::explain(config, &preset).join("\n"));
        }
        PresetCommand::Apply { name, write } => {
            let preset = v8q::preset::find(&name)
                .ok_or_else(|| anyhow::anyhow!("unknown preset: {name}"))?;
            for line in v8q::preset::describe_diff(config, &preset) {
                println!("{line}");
            }
            if write {
                let mut config = config.clone();
                let backup = backup_config_file()?;
                v8q::preset::apply(&mut config, &preset);
                v8q::save_config(&config)?;
                println!("Applied preset: {}", preset.name);
                println!("Backup: {}", backup.display());
            } else {
                println!("Dry run only. Re-run with --write to apply.");
            }
        }
    }
    Ok(())
}

fn handle_mode(command: ModeCommand, config: &v8q::Config) -> Result<()> {
    match command {
        ModeCommand::Show => println!("{}", config.ui.mode),
        ModeCommand::Beginner | ModeCommand::Advanced => {
            let mut next = config.clone();
            next.ui.mode = match command {
                ModeCommand::Beginner => "beginner",
                ModeCommand::Advanced => "advanced",
                ModeCommand::Show => unreachable!(),
            }
            .to_string();
            let backup = backup_config_file()?;
            v8q::save_config(&next)?;
            println!("Mode: {}", next.ui.mode);
            println!("Backup: {}", backup.display());
        }
    }
    Ok(())
}

fn handle_windows(json: bool) -> Result<()> {
    let windows = v8q::window::list_hyprland_windows()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&windows)?);
    } else {
        for window in windows {
            println!(
                "{} | class={} app_id={} workspace={} pid={} address={} geometry={}",
                window.title,
                window.class,
                window.app_id,
                window.workspace,
                window
                    .pid
                    .map(|pid| pid.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                window.address,
                window.geometry()
            );
        }
    }
    Ok(())
}

fn handle_window(command: WindowCommand, config: &v8q::Config) -> Result<()> {
    match command {
        WindowCommand::Select {
            title,
            app_id,
            class,
            interactive,
            follow,
        } => {
            let windows = v8q::window::list_hyprland_windows()?;
            let selected = if interactive {
                select_window_interactive(&windows)
            } else if title.is_none() && app_id.is_none() && class.is_none() {
                v8q::window::select_window(&windows, None, None, None)
                    .or_else(|_| active_window_from_hyprctl())
            } else {
                v8q::window::select_window(
                    &windows,
                    title.as_deref(),
                    class.as_deref(),
                    app_id.as_deref(),
                )
            }?;
            let mut next = config.clone();
            v8q::window::apply_selected_window(&mut next, &selected);
            next.capture
                .get_or_insert_with(v8q::config::CaptureConfig::default)
                .window
                .get_or_insert_with(v8q::config::CaptureWindowConfig::default)
                .follow = follow;
            let backup = backup_config_file()?;
            v8q::save_config(&next)?;
            println!("Selected window:");
            println!("  title: {}", selected.title);
            println!("  class: {}", selected.class);
            println!("  address: {}", selected.address);
            println!("  geometry: {}", selected.geometry());
            println!("Backup: {}", backup.display());
        }
        WindowCommand::Clear => {
            let mut next = config.clone();
            next.capture
                .get_or_insert_with(v8q::config::CaptureConfig::default)
                .window = Some(v8q::config::CaptureWindowConfig::default());
            next.capture_window = None;
            next.wl_screenrec.capture_mode = "output".to_string();
            next.wl_screenrec.geometry.clear();
            let backup = backup_config_file()?;
            v8q::save_config(&next)?;
            println!("Window capture cleared.");
            println!("Backup: {}", backup.display());
        }
        WindowCommand::Show => {
            let window = config.effective_capture_window();
            println!("enabled: {}", window.enabled);
            println!("title: {}", window.title);
            println!("class: {}", window.class);
            println!("address: {}", window.address);
            println!("geometry: {}", window.geometry);
            println!("follow: {}", window.follow);
        }
    }
    Ok(())
}

fn select_window_interactive(
    windows: &[v8q::window::WindowInfo],
) -> Result<v8q::window::WindowInfo> {
    if windows.is_empty() {
        anyhow::bail!("no Hyprland windows found");
    }
    for (index, window) in windows.iter().enumerate() {
        println!(
            "{}: {} | class={} app_id={} address={} geometry={}",
            index + 1,
            window.title,
            window.class,
            window.app_id,
            window.address,
            window.geometry()
        );
    }
    print!("Select window number: ");
    std::io::stdout().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let index = input
        .trim()
        .parse::<usize>()
        .context("invalid window number")?;
    windows
        .get(index.saturating_sub(1))
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("window number out of range"))
}

fn active_window_from_hyprctl() -> Result<v8q::window::WindowInfo> {
    let output = ProcessCommand::new("hyprctl")
        .args(["activewindow", "-j"])
        .output()
        .context("failed to run hyprctl activewindow -j")?;
    if !output.status.success() {
        anyhow::bail!(
            "hyprctl activewindow -j failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let json = format!("[{}]", String::from_utf8_lossy(&output.stdout));
    let windows = v8q::window::parse_hyprctl_clients_json(&json)?;
    windows
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("could not read active Hyprland window"))
}

fn backup_config_file() -> Result<PathBuf> {
    let path = v8q::config_path()?;
    let backup = path.with_extension(format!(
        "toml.bak-{}-{}",
        chrono::Local::now().format("%Y%m%d-%H%M%S"),
        std::process::id()
    ));
    std::fs::copy(&path, &backup)?;
    Ok(backup)
}

fn handle_service(command: ServiceCommand) -> Result<()> {
    match command {
        ServiceCommand::Print => print!("{}", v8q::service::service_file()),
        ServiceCommand::Install => println!("Installed {}", v8q::service::install()?.display()),
        ServiceCommand::Uninstall => println!("Removed {}", v8q::service::uninstall()?.display()),
        ServiceCommand::Start => print!("{}", v8q::service::systemctl(&["start", "v8q.service"])?),
        ServiceCommand::Stop => print!("{}", v8q::service::systemctl(&["stop", "v8q.service"])?),
        ServiceCommand::Status => {
            print!("{}", v8q::service::systemctl(&["status", "v8q.service"])?)
        }
        ServiceCommand::Enable => {
            print!("{}", v8q::service::systemctl(&["enable", "v8q.service"])?)
        }
        ServiceCommand::Disable => {
            print!("{}", v8q::service::systemctl(&["disable", "v8q.service"])?)
        }
    }
    Ok(())
}

fn handle_debug(command: DebugCommand, config: &v8q::Config) -> Result<()> {
    match command {
        DebugCommand::Info => debug_info(config),
        DebugCommand::WlScreenrec { test_run } => {
            if let Some(seconds) = test_run {
                debug_wl_screenrec_test_run(config, seconds)
            } else {
                debug_wl_screenrec(config)
            }
        }
        DebugCommand::Paths => debug_paths(config),
        DebugCommand::Window => debug_window(config),
        DebugCommand::Audio => handle_audio(AudioCommand::Sources),
    }
}

fn debug_info(config: &v8q::Config) -> Result<()> {
    println!("V8Q debug info\n");
    println!("version: {}", env!("CARGO_PKG_VERSION"));
    println!("os: {}", std::env::consts::OS);
    println!(
        "session: {}",
        std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".to_string())
    );
    println!(
        "desktop: {}",
        std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "unknown".to_string())
    );
    println!("backend: {}", config.effective_backend()?.as_str());
    println!("config: {}", v8q::config_path()?.display());
    println!("output: {}", config.paths.output_dir_path().display());
    println!("buffer: {}", config.paths.buffer_dir_path().display());
    println!("logs: {}", v8q::paths::logs_dir().display());
    let status = v8q::get_status(config)?;
    println!("running: {}", status.is_running);
    if let Some(pid) = status.pid {
        println!("pid: {pid}");
    }
    Ok(())
}

fn debug_paths(config: &v8q::Config) -> Result<()> {
    println!("V8Q paths\n");
    println!("config: {}", v8q::config_path()?.display());
    println!("buffer: {}", config.paths.buffer_dir_path().display());
    println!("output: {}", config.paths.output_dir_path().display());
    println!("pid: {}", config.paths.pid_file_path().display());
    println!("logs: {}", v8q::paths::logs_dir().display());
    println!("history: {}", config.wl_screenrec_history_file().display());
    Ok(())
}

fn debug_window(config: &v8q::Config) -> Result<()> {
    println!("V8Q window debug\n");
    println!(
        "hyprland_env: {}",
        std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok()
    );
    println!(
        "hyprctl: {}",
        if v8q::doctor::command_path("hyprctl").is_some() {
            "found"
        } else {
            "missing"
        }
    );
    match v8q::window::list_hyprland_windows() {
        Ok(windows) => println!("windows: {}", windows.len()),
        Err(error) => println!("windows_error: {error:#}"),
    }
    let capture_window = config.effective_capture_window();
    println!("selected_enabled: {}", capture_window.enabled);
    println!("selected_title: {}", capture_window.title);
    println!("selected_class: {}", capture_window.class);
    println!("selected_address: {}", capture_window.address);
    println!("selected_geometry: {}", capture_window.geometry);
    println!("selected_follow: {}", capture_window.follow);
    let help = v8q::doctor::wl_screenrec_help_text().unwrap_or_default();
    println!(
        "wl_screenrec_geometry_support: {}",
        help.contains("--geometry")
    );
    Ok(())
}

fn handle_audio(command: AudioCommand) -> Result<()> {
    match command {
        AudioCommand::Sources => {
            let output = ProcessCommand::new("pactl")
                .args(["list", "short", "sources"])
                .output()
                .context("failed to run pactl list short sources")?;
            if !output.status.success() {
                anyhow::bail!("pactl failed:\n{}", String::from_utf8_lossy(&output.stderr));
            }
            println!("PipeWire/PulseAudio sources:\n");
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                if line.contains(".monitor") {
                    println!("* {line}");
                } else {
                    println!("  {line}");
                }
            }
            println!(
                "\nMonitor sources are usually best for replay audio. Copy one .monitor source into `[wl_screenrec] audio_device`."
            );
        }
    }
    Ok(())
}

fn debug_wl_screenrec(config: &v8q::Config) -> Result<()> {
    let backend = config.effective_backend()?;
    let buffer_dir = config.paths.buffer_dir_path();
    let history_file = config.wl_screenrec_history_file();
    let pid_file = config.paths.pid_file_path();
    let output_dir = config.paths.output_dir_path();
    let _ = std::fs::create_dir_all(&buffer_dir);
    let (command, help_result) = match v8q::wl_screenrec::command_for_config(config) {
        Ok((command, help)) => (Some(command), Ok(help)),
        Err(error) => (None, Err(error)),
    };
    let (pid, warnings) = v8q::process::recorder_pid_checked(&pid_file, Some("wl-screenrec"))?;

    println!("V8Q wl-screenrec debug\n");
    println!("config_path: {}", v8q::config_path()?.display());
    println!("backend_resolved: {}", backend.as_str());
    println!("buffer_dir: {}", buffer_dir.display());
    println!("expected_history_file: {}", history_file.display());
    println!("pid_file: {}", pid_file.display());
    println!("output_dir: {}", output_dir.display());
    println!("capture_mode: {}", config.wl_screenrec.capture_mode);
    println!(
        "auto_select_focused_output: {}",
        config.wl_screenrec.auto_select_focused_output
    );
    if !config.wl_screenrec.output.trim().is_empty() {
        println!("configured_output: {}", config.wl_screenrec.output);
    }
    if !config.wl_screenrec.geometry.trim().is_empty() {
        println!("configured_geometry: {}", config.wl_screenrec.geometry);
    }
    if config.wl_screenrec.output.trim().is_empty()
        && config.wl_screenrec.geometry.trim().is_empty()
        && matches!(
            config.wl_screenrec.capture_mode.as_str(),
            "output" | "monitor"
        )
    {
        if let Ok(output) = ProcessCommand::new("hyprctl")
            .args(["monitors", "-j"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            let names = hyprctl_monitor_names(&text);
            if !names.is_empty() {
                println!("hyprland_outputs: {}", names.join(", "));
                if names.len() > 1 {
                    println!(
                        "warning: multiple outputs detected; set `[wl_screenrec] output = \"{}\"` or another listed output",
                        names[0]
                    );
                }
            }
        }
    }
    if let Some(command) = command {
        println!("command: {}", command.join(" "));
    }
    match help_result {
        Ok(help) => {
            println!("wl_screenrec_help_detected: true");
            print_supported_flags(&help);
        }
        Err(error) => {
            println!("wl_screenrec_help_detected: false");
            println!("wl_screenrec_help_error: {error:#}");
        }
    }
    println!("process_exists: {}", pid.is_some());
    for warning in warnings {
        println!("warning: {warning}");
    }
    if let Some(pid) = pid {
        println!("pid: {pid}");
        println!(
            "proc_cmdline: {}",
            v8q::process::proc_cmdline(pid).unwrap_or_else(|| "<unavailable>".to_string())
        );
    }
    println!("history_exists: {}", history_file.exists());
    println!(
        "history_size_bytes: {}",
        std::fs::metadata(&history_file)
            .map(|metadata| metadata.len())
            .unwrap_or(0)
    );
    println!("buffer_permissions: {}", permissions_summary(&buffer_dir));
    println!("\nLast 100 wl-screenrec log lines:");
    for line in v8q::wl_screenrec::tail_log(&v8q::wl_screenrec::log_file_path(), 100) {
        println!("{line}");
    }
    Ok(())
}

fn debug_wl_screenrec_test_run(config: &v8q::Config, seconds: u64) -> Result<()> {
    println!("Stopping any current recorder before test-run.");
    let _ = v8q::stop_recorder(config);

    let buffer_dir = config.paths.buffer_dir_path();
    let history_file = config.wl_screenrec_history_file();
    std::fs::create_dir_all(&buffer_dir)?;
    let _ = std::fs::remove_file(&history_file);

    let (command, _) = v8q::wl_screenrec::command_for_config(config)?;
    let (program, args) = command
        .split_first()
        .ok_or_else(|| anyhow::anyhow!("wl-screenrec command is empty"))?;
    let log_file = v8q::paths::logs_dir().join("wl-screenrec_test.log");
    v8q::paths::ensure_parent_dir(&log_file)?;
    let _ = std::fs::remove_file(&log_file);
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)?;
    let stderr = stdout.try_clone()?;

    println!("test_command: {}", command.join(" "));
    let mut command = ProcessCommand::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    detach_recorder_process_group(&mut command);

    let mut child = command
        .spawn()
        .context("failed to spawn wl-screenrec test-run")?;
    println!("test_pid: {}", child.id());
    std::thread::sleep(Duration::from_secs(seconds.max(1)));
    v8q::process::send_signal(child.id(), "USR1")?;

    let appeared = wait_for_debug_history(&history_file, Duration::from_secs(10));
    let size = std::fs::metadata(&history_file)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    println!("history_appeared: {appeared}");
    println!("history_size_bytes: {size}");

    let _ = v8q::process::send_signal(child.id(), "TERM");
    let _ = child.wait();

    println!("\nTest log:");
    let mut contents = String::new();
    let _ = std::fs::File::open(&log_file).and_then(|mut file| file.read_to_string(&mut contents));
    print!("{contents}");

    if !appeared || size == 0 {
        anyhow::bail!(
            "wl-screenrec test-run did not produce a non-empty history file at {}",
            history_file.display()
        );
    }
    Ok(())
}

fn detach_recorder_process_group(command: &mut ProcessCommand) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
}

fn print_supported_flags(help: &str) {
    let flags = [
        "--history",
        "--filename",
        "--max-fps",
        "--ffmpeg-encoder",
        "--audio",
        "--audio-device",
        "--audio-backend",
        "--ffmpeg-encoder-options",
        "--bitrate",
        "--output",
        "--geometry",
    ];
    println!("supported_flags:");
    for flag in flags {
        println!("  {flag}: {}", help.contains(flag));
    }
}

fn wait_for_debug_history(path: &Path, timeout: Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if std::fs::metadata(path)
            .map(|metadata| metadata.len() > 0)
            .unwrap_or(false)
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    false
}

fn permissions_summary(path: &Path) -> String {
    match std::fs::metadata(path) {
        Ok(metadata) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                format!(
                    "mode={:o}, readonly={}",
                    metadata.permissions().mode() & 0o777,
                    metadata.permissions().readonly()
                )
            }
            #[cfg(not(unix))]
            {
                format!("readonly={}", metadata.permissions().readonly())
            }
        }
        Err(error) => format!("unavailable: {error}"),
    }
}

fn hyprctl_monitor_names(text: &str) -> Vec<String> {
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

fn run_foreground(config: &v8q::Config) -> Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let signal = Arc::clone(&running);
    ctrlc::set_handler(move || signal.store(false, Ordering::SeqCst))?;
    while running.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_secs(1));
    }
    let _ = v8q::stop_recorder(config);
    Ok(())
}

fn print_shell_setup(write: bool) -> Result<()> {
    let in_path = cargo_bin_in_path();
    if in_path && !write {
        println!("PATH OK: ~/.cargo/bin is already available.");
        return Ok(());
    }
    let shell = std::env::var("SHELL").unwrap_or_default();
    let (file, line) = if shell.contains("fish") {
        (
            "~/.config/fish/config.fish",
            "fish_add_path $HOME/.cargo/bin",
        )
    } else if shell.contains("zsh") {
        ("~/.zshrc", "export PATH=\"$HOME/.cargo/bin:$PATH\"")
    } else {
        ("~/.bashrc", "export PATH=\"$HOME/.cargo/bin:$PATH\"")
    };
    println!("Add ~/.cargo/bin to PATH:");
    println!("echo '{}' >> {}", line, file);
    println!("Then restart your shell or source the file.");
    if write {
        let path = v8q::paths::expand_tilde(file);
        v8q::paths::ensure_parent_dir(&path)?;
        use std::io::Write;
        writeln!(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?,
            "{line}"
        )?;
    }
    Ok(())
}

fn cargo_bin_in_path() -> bool {
    let cargo_bin = v8q::paths::expand_tilde("~/.cargo/bin");
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|path| path == cargo_bin))
        .unwrap_or(false)
}

fn print_hyprland_setup(write: bool) -> Result<()> {
    let block = "bind = SUPER_SHIFT, R, exec, v8q save\nbind = SUPER_SHIFT, S, exec, v8q start\nbind = SUPER_SHIFT, X, exec, v8q stop\nbind = SUPER_SHIFT, D, exec, v8q doctor\n";
    println!("Hyprland binds:\n{block}");
    if write {
        let path = v8q::paths::expand_tilde("~/.config/hypr/hyprland.conf");
        use std::io::Write;
        writeln!(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?,
            "\n# V8Q\n{block}"
        )?;
    }
    Ok(())
}

fn ensure_clip_in_output(config: &v8q::Config, path: &Path, force_external: bool) -> Result<()> {
    if force_external {
        return Ok(());
    }
    let output = config.paths.output_dir_path().canonicalize()?;
    let clip = path.canonicalize()?;
    if !clip.starts_with(output) {
        anyhow::bail!("refusing to modify clip outside output_dir; use --force-external");
    }
    Ok(())
}

fn delete_clip(config: &v8q::Config, path: &Path, force_external: bool) -> Result<()> {
    ensure_clip_in_output(config, path, force_external)?;
    std::fs::remove_file(path)?;
    Ok(())
}

fn log_files(backend: Option<&str>) -> Vec<PathBuf> {
    match backend {
        Some("wl-screenrec") => vec![v8q::paths::log_file_for_backend("wl-screenrec")],
        Some("ffmpeg") => vec![v8q::paths::log_file_for_backend("ffmpeg")],
        _ => vec![
            v8q::paths::log_file_for_backend("wl-screenrec"),
            v8q::paths::log_file_for_backend("ffmpeg"),
            v8q::paths::latest_log_file(),
        ],
    }
}

fn clean_logs(_older_than: Option<&str>) -> Result<usize> {
    let dir = v8q::paths::logs_dir();
    if !dir.exists() {
        return Ok(0);
    }
    let mut removed = 0;
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("log") {
            std::fs::remove_file(path)?;
            removed += 1;
        }
    }
    Ok(removed)
}

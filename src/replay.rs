use std::cmp::Reverse;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context};
use chrono::Local;

use crate::config::{CaptureBackend, Config};
use crate::lock::FileLock;
use crate::process;
use crate::{CleanResult, ClipsResult, LogsResult, SaveResult, StatusInfo};

const MIN_VALID_HISTORY_BYTES: u64 = 1024;

pub fn status_info(config: &Config) -> anyhow::Result<StatusInfo> {
    let pid_file = config.paths.pid_file_path();
    let buffer_dir = config.paths.buffer_dir_path();
    let output_dir = config.paths.output_dir_path();
    let backend = config.effective_backend()?;

    let expected_backend = backend.as_str();
    let (running_pid, mut warnings) =
        process::recorder_pid_checked(&pid_file, Some(expected_backend))?;
    let process_command = running_pid.and_then(process::proc_cmdline);
    let metadata = process::read_pid_meta(&pid_file).ok().flatten();
    if config.paths.buffer_dir_path() == Path::new("/tmp/v8q-buffer") {
        warnings.push(format!(
            "Using legacy buffer_dir /tmp/v8q-buffer. Recommended: {}",
            crate::paths::default_buffer_dir().display()
        ));
    }
    let expected_history_file =
        (backend == CaptureBackend::WlScreenrec).then_some(config.wl_screenrec_history_file());
    let (history_exists, history_size_bytes) = expected_history_file
        .as_ref()
        .map(|path| {
            let metadata = fs::metadata(path).ok();
            (
                Some(metadata.is_some()),
                metadata.map(|metadata| metadata.len()),
            )
        })
        .unwrap_or((None, None));
    let history_valid = match (history_exists, history_size_bytes) {
        (Some(false), _) => None,
        (Some(true), Some(size)) => Some(size >= MIN_VALID_HISTORY_BYTES),
        (Some(true), None) => None,
        _ => None,
    };
    if backend == CaptureBackend::WlScreenrec {
        if let (Some(true), Some(size)) = (history_exists, history_size_bytes) {
            if size < MIN_VALID_HISTORY_BYTES {
                warnings.push(format!(
                    "History file exists but is probably not a valid replay yet: size is only {size} bytes."
                ));
            }
        }
    }
    let log_tail = if backend == CaptureBackend::WlScreenrec {
        crate::wl_screenrec::tail_log(&crate::wl_screenrec::log_file_path(), 30)
            .into_iter()
            .map(|line| strip_ansi(&line))
            .collect()
    } else {
        Vec::new()
    };
    let last_error_lines = log_tail
        .iter()
        .filter(|line| is_relevant_log_error(line))
        .rev()
        .take(3)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    let latest_clip = crate::latest_clip(config).ok().flatten();
    let capture_target = capture_target_summary(config);

    Ok(StatusInfo {
        is_running: running_pid.is_some(),
        pid: running_pid,
        backend: backend.as_str().to_string(),
        config_path: Config::config_file_path()?,
        buffer_dir: buffer_dir.clone(),
        output_dir,
        latest_clip,
        detected_preset: crate::preset::detect(config).map(ToString::to_string),
        capture_target,
        replay_duration: config.recording.duration_seconds,
        segment_duration: config.recording.segment_seconds,
        fps: config.recording.fps,
        encoder: config.effective_encoder()?.to_string(),
        bitrate: config.effective_bitrate()?.to_string(),
        audio_enabled: (backend == CaptureBackend::WlScreenrec)
            .then_some(config.wl_screenrec.audio),
        segment_count: backend
            .is_segmented()
            .then_some(list_segments(&buffer_dir)?.len()),
        history_file: expected_history_file.clone(),
        last_log_lines: log_lines(config)?,
        process_command,
        metadata,
        warnings,
        error: None,
        log_tail,
        last_error_lines,
        log_file: (backend == CaptureBackend::WlScreenrec).then(crate::wl_screenrec::log_file_path),
        expected_history_file,
        history_exists,
        history_size_bytes,
        history_valid,
    })
}

fn capture_target_summary(config: &Config) -> String {
    let window = config.effective_capture_window();
    if window.enabled {
        return format!(
            "window title='{}' class='{}' address='{}' geometry='{}' follow={}",
            window.title, window.class, window.address, window.geometry, window.follow
        );
    }
    match config.wl_screenrec.capture_mode.as_str() {
        "geometry" => format!("geometry {}", config.wl_screenrec.geometry),
        "active-window" | "window" => "active window".to_string(),
        _ if !config.wl_screenrec.output.trim().is_empty() => {
            format!("output {}", config.wl_screenrec.output)
        }
        _ => "fullscreen/focused output".to_string(),
    }
}

pub fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if ('\u{40}'..='\u{7e}').contains(&next) {
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

pub fn is_relevant_log_error(line: &str) -> bool {
    line.contains("ERROR")
        || line.contains("Error:")
        || line.contains("error:")
        || line.contains("panic")
        || line.contains("failed")
        || line.contains("Failed")
}

pub fn last_relevant_log_error<'a, I>(lines: I) -> Option<String>
where
    I: IntoIterator<Item = &'a str>,
{
    lines
        .into_iter()
        .filter(|line| is_relevant_log_error(line))
        .last()
        .map(ToString::to_string)
}

pub fn save(config: &Config) -> anyhow::Result<SaveResult> {
    save_with_options(config, &SaveOptions::default())
}

#[derive(Debug, Clone, Default)]
pub struct SaveOptions {
    pub name: Option<String>,
    pub duration_seconds: Option<u64>,
}

pub fn save_with_options(config: &Config, options: &SaveOptions) -> anyhow::Result<SaveResult> {
    let _save_lock = FileLock::acquire_for(
        &save_lock_file(),
        "save",
        "another save is already in progress",
    )?;
    if config.uses_wl_screenrec()? {
        return save_wl_screenrec(config, options);
    }

    let buffer_dir = config.paths.buffer_dir_path();
    let output_dir = config.paths.output_dir_path();
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("failed to create output directory {}", output_dir.display()))?;

    let mut segments = list_segments(&buffer_dir)?;
    segments.sort_by_key(|segment| segment.modified);

    let duration = options
        .duration_seconds
        .unwrap_or(config.recording.duration_seconds);
    let needed = duration
        .max(1)
        .div_ceil(config.recording.segment_seconds.max(1)) as usize;
    if segments.len() < needed {
        return Err(anyhow!(
            "not enough segments yet: have {}, need {} for {} seconds",
            segments.len(),
            needed,
            duration
        ));
    }

    let selected: Vec<PathBuf> = segments
        .into_iter()
        .rev()
        .take(needed)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|segment| segment.path)
        .collect();

    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
    let output_file = unique_output_path(&output_dir, options.name.as_deref());
    let concat_file = buffer_dir.join(format!("concat_{timestamp}.txt"));
    write_concat_file(&concat_file, &selected)?;

    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-f")
        .arg("concat")
        .arg("-safe")
        .arg("0")
        .arg("-i")
        .arg(&concat_file)
        .arg("-c")
        .arg("copy")
        .arg(&output_file)
        .status()
        .with_context(|| "failed to run ffmpeg concat command")?;

    let _ = fs::remove_file(&concat_file);

    if !status.success() {
        return Err(anyhow!("ffmpeg failed while saving replay clip"));
    }

    Ok(SaveResult {
        output_file,
        duration_seconds: duration,
        backend: config.effective_backend()?.as_str().to_string(),
    })
}

fn save_wl_screenrec(config: &Config, options: &SaveOptions) -> anyhow::Result<SaveResult> {
    let pid_file = config.paths.pid_file_path();
    let output_dir = config.paths.output_dir_path();
    let history_file = config.wl_screenrec_history_file();
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("failed to create output directory {}", output_dir.display()))?;

    let (pid, warnings) = process::recorder_pid_checked(&pid_file, Some("wl-screenrec"))?;
    let pid = pid.ok_or_else(|| recorder_not_running_error(config, &warnings))?;
    let cmdline = process::proc_cmdline(pid).unwrap_or_else(|| "<unavailable>".to_string());

    process::send_signal(pid, "USR1")?;
    wait_for_history_file(
        config,
        pid,
        &cmdline,
        &history_file,
        Duration::from_secs(15),
    )
    .map_err(|error| {
        anyhow!(
            "{error}\n{}",
            wl_screenrec_failure_context(config, pid, &cmdline, &history_file, &warnings)
        )
    })?;

    process::send_signal(pid, "TERM")
        .with_context(|| format!("failed to stop wl-screenrec PID {pid} after history flush"))?;
    wait_for_process_exit(pid, Duration::from_secs(5))
        .with_context(|| format!("wl-screenrec PID {pid} did not exit after history flush"))?;
    process::remove_pid_files(&pid_file)?;
    wait_for_stable_file(&history_file, Duration::from_secs(4))?;

    let output_file = unique_output_path(&output_dir, options.name.as_deref());
    move_file(&history_file, &output_file)?;

    if let Err(error) = crate::wl_screenrec::start(config) {
        eprintln!("WARN: saved replay, but failed to restart wl-screenrec recorder: {error:#}");
    }

    Ok(SaveResult {
        output_file,
        duration_seconds: options
            .duration_seconds
            .unwrap_or(config.recording.duration_seconds),
        backend: "wl-screenrec".to_string(),
    })
}

pub fn clean(config: &Config) -> anyhow::Result<CleanResult> {
    let buffer_dir = config.paths.buffer_dir_path();
    if !buffer_dir.exists() {
        return Ok(CleanResult {
            buffer_dir,
            removed_files: 0,
        });
    }

    let mut removed = 0usize;
    for entry in fs::read_dir(&buffer_dir)
        .with_context(|| format!("failed to read buffer directory {}", buffer_dir.display()))?
    {
        let path = entry?.path();
        if path.is_file() && is_cleanable_buffer_file(&path) {
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove {}", path.display()))?;
            removed += 1;
        }
    }

    Ok(CleanResult {
        buffer_dir,
        removed_files: removed,
    })
}

#[derive(Debug)]
struct Segment {
    path: PathBuf,
    modified: std::time::SystemTime,
}

fn list_segments(buffer_dir: &Path) -> anyhow::Result<Vec<Segment>> {
    if !buffer_dir.exists() {
        return Ok(Vec::new());
    }

    let mut segments = Vec::new();
    for entry in fs::read_dir(buffer_dir)
        .with_context(|| format!("failed to read buffer directory {}", buffer_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !is_segment_file(&path) {
            continue;
        }

        let metadata = entry
            .metadata()
            .with_context(|| format!("failed to read metadata for {}", path.display()))?;
        segments.push(Segment {
            path,
            modified: metadata
                .modified()
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
        });
    }

    segments.sort_by_key(|segment| Reverse(segment.modified));
    Ok(segments)
}

fn is_segment_file(path: &Path) -> bool {
    path.is_file()
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.starts_with("seg_") && name.ends_with(".mkv"))
            .unwrap_or(false)
}

fn is_cleanable_buffer_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| {
            (name.starts_with("seg_") && name.ends_with(".mkv"))
                || (name.starts_with("concat_") && name.ends_with(".txt"))
                || name == "history.mkv"
        })
        .unwrap_or(false)
}

fn write_concat_file(path: &Path, segments: &[PathBuf]) -> anyhow::Result<()> {
    let mut file = fs::File::create(path)
        .with_context(|| format!("failed to create concat file {}", path.display()))?;

    for segment in segments {
        writeln!(file, "file '{}'", escape_concat_path(segment))
            .with_context(|| format!("failed to write concat file {}", path.display()))?;
    }

    Ok(())
}

fn escape_concat_path(path: &Path) -> String {
    path.to_string_lossy().replace('\'', "'\\''")
}

fn wait_for_history_file(
    config: &Config,
    pid: u32,
    cmdline: &str,
    path: &Path,
    timeout: Duration,
) -> anyhow::Result<()> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if !process::is_process_running(pid) {
            let _ = process::remove_pid_files(&config.paths.pid_file_path());
            return Err(anyhow!(
                "recorder process exited before save; wl-screenrec crashed or stopped before writing history file; pid={pid}; cmdline={cmdline}"
            ));
        }
        match fs::metadata(path) {
            Ok(metadata) if metadata.len() >= MIN_VALID_HISTORY_BYTES => return Ok(()),
            Ok(_) => {}
            Err(_) => {}
        }
        thread::sleep(Duration::from_millis(150));
    }
    Err(anyhow!(
        "timed out waiting for wl-screenrec to write {}",
        path.display()
    ))
}

fn wait_for_process_exit(pid: u32, timeout: Duration) -> anyhow::Result<()> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if !process::is_process_running(pid) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }
    Err(anyhow!("PID {pid} is still running"))
}

fn recorder_not_running_error(config: &Config, warnings: &[String]) -> anyhow::Error {
    let log_tail = crate::wl_screenrec::tail_log(&crate::wl_screenrec::log_file_path(), 100)
        .into_iter()
        .map(|line| strip_ansi(&line))
        .collect::<Vec<_>>();
    let warning_text = if warnings.is_empty() {
        "<none>".to_string()
    } else {
        warnings.join("; ")
    };
    anyhow!(
        "recorder process exited before save; wl-screenrec crashed or is not running.\n\
         V8Q detected it and preserved the log.\n\
         warnings: {warning_text}\n\
         expected history file: {}\n\
         log tail:\n{}\n\
         Recommended actions:\n\
           - Run `v8q debug wl-screenrec --test-run 5`.\n\
           - Run `v8q preset apply beginner-safe --write` or `v8q preset apply wl-screenrec-nvidia-compat --write`.\n\
           - Reduce FPS/bitrate if the backend still exits.\n\
           - Test a custom/wf-recorder backend if wl-screenrec keeps crashing on this NVIDIA/Hyprland setup.",
        config.wl_screenrec_history_file().display(),
        if log_tail.is_empty() {
            "<empty>".to_string()
        } else {
            log_tail.join("\n")
        },
    )
}

fn wl_screenrec_failure_context(
    config: &Config,
    pid: u32,
    cmdline: &str,
    history_file: &Path,
    warnings: &[String],
) -> String {
    let buffer_dir = config.paths.buffer_dir_path();
    let listing = fs::read_dir(&buffer_dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| {
                    let path = entry.path();
                    let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    format!("  {} ({} bytes)", path.display(), size)
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_else(|error| format!("  <failed to list buffer dir: {error}>"));
    let log_tail = crate::wl_screenrec::tail_log(&crate::wl_screenrec::log_file_path(), 100)
        .into_iter()
        .map(|line| strip_ansi(&line))
        .collect::<Vec<_>>();
    format!(
        "wl-screenrec save debug context:\n  pid: {pid}\n  cmdline: {cmdline}\n  expected history file: {}\n  buffer dir: {}\n  process alive: {}\n  warnings: {}\n  buffer listing:\n{}\n  log tail:\n{}\nRecommended actions:\n  - Run `v8q debug wl-screenrec --test-run 5`.\n  - Run `v8q preset apply beginner-safe --write` for the safest first-run config.\n  - Run `v8q preset apply wl-screenrec-nvidia-compat --write` for the 60 FPS NVIDIA compatibility path.\n  - Reduce FPS/bitrate if the backend still exits.\n  - If multiple displays are enabled, set `[wl_screenrec] output = \"DP-1\"` or another output from `hyprctl monitors -j`.\n  - Test a custom/wf-recorder backend if wl-screenrec keeps crashing on this NVIDIA/Hyprland setup.",
        history_file.display(),
        buffer_dir.display(),
        process::is_process_running(pid),
        if warnings.is_empty() { "<none>".to_string() } else { warnings.join("; ") },
        listing,
        if log_tail.is_empty() { "<empty>".to_string() } else { log_tail.join("\n") },
    )
}

fn wait_for_stable_file(path: &Path, timeout: Duration) -> anyhow::Result<()> {
    let deadline = Instant::now() + timeout;
    let mut last_size = None;
    let mut stable_ticks = 0u8;

    while Instant::now() < deadline {
        if let Ok(metadata) = fs::metadata(path) {
            let size = metadata.len();
            if size >= MIN_VALID_HISTORY_BYTES && Some(size) == last_size {
                stable_ticks += 1;
                if stable_ticks >= 2 {
                    return Ok(());
                }
            } else {
                stable_ticks = 0;
                last_size = Some(size);
            }
        }

        thread::sleep(Duration::from_millis(250));
    }

    Err(anyhow!(
        "timed out waiting for wl-screenrec to finish {}",
        path.display()
    ))
}

fn move_file(from: &Path, to: &Path) -> anyhow::Result<()> {
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(rename_error) => {
            fs::copy(from, to).with_context(|| {
                format!(
                    "failed to copy {} to {} after rename failed: {rename_error}",
                    from.display(),
                    to.display()
                )
            })?;
            fs::remove_file(from)
                .with_context(|| format!("failed to remove {}", from.display()))?;
            Ok(())
        }
    }
}

fn unique_output_path(output_dir: &Path, name: Option<&str>) -> PathBuf {
    let first = output_dir.join(crate::build_clip_filename(name));
    if !first.exists() {
        return first;
    }
    let stem = first
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("v8q_clip")
        .to_string();
    for counter in 1..1000 {
        let candidate = output_dir.join(format!("{stem}_{counter}.mkv"));
        if !candidate.exists() {
            return candidate;
        }
    }
    output_dir.join(format!("{stem}_{}.mkv", Local::now().timestamp_millis()))
}

pub fn logs(config: &Config) -> anyhow::Result<LogsResult> {
    Ok(LogsResult {
        logs: log_lines(config)?,
    })
}

fn log_lines(_config: &Config) -> anyhow::Result<Vec<(PathBuf, Vec<String>)>> {
    let mut logs = Vec::new();
    for path in [
        crate::paths::log_file_for_backend("wl-screenrec"),
        crate::paths::log_file_for_backend("ffmpeg"),
        crate::paths::latest_log_file(),
    ] {
        if path.exists() {
            logs.push((path.clone(), read_log_tail(&path)?));
        }
    }
    Ok(logs)
}

fn read_log_tail(path: &Path) -> anyhow::Result<Vec<String>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read log file {}", path.display()))?;
    Ok(contents
        .lines()
        .rev()
        .take(10)
        .map(strip_ansi)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect())
}

fn save_lock_file() -> PathBuf {
    crate::paths::default_save_lock_file()
}

pub fn list_clips(config: &Config) -> anyhow::Result<ClipsResult> {
    let output_dir = config.paths.output_dir_path();
    if !output_dir.exists() {
        return Ok(ClipsResult { clips: Vec::new() });
    }

    let mut clips = Vec::new();
    for entry in fs::read_dir(&output_dir)
        .with_context(|| format!("failed to read output directory {}", output_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with("v8q_") && name.ends_with(".mkv"))
                .unwrap_or(false)
        {
            let modified = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            clips.push((modified, path));
        }
    }

    clips.sort_by_key(|(modified, _)| *modified);
    Ok(ClipsResult {
        clips: clips.into_iter().map(|(_, path)| path).collect(),
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::lock::FileLock;

    use super::{
        escape_concat_path, is_cleanable_buffer_file, is_segment_file, strip_ansi,
        wl_screenrec_failure_context,
    };

    #[test]
    fn detects_segment_files() {
        let dir = std::env::temp_dir().join(format!("v8q-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let segment = dir.join("seg_000001.mkv");
        let concat = dir.join("concat.txt");
        let wrong_extension = dir.join("seg_000001.mp4");
        std::fs::write(&segment, []).unwrap();
        std::fs::write(&concat, []).unwrap();
        std::fs::write(&wrong_extension, []).unwrap();

        assert!(is_segment_file(&segment));
        assert!(!is_segment_file(&concat));
        assert!(!is_segment_file(&wrong_extension));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn escapes_single_quotes_for_concat_file() {
        assert_eq!(
            escape_concat_path(Path::new("/tmp/a'b.mkv")),
            "/tmp/a'\\''b.mkv"
        );
    }

    #[test]
    fn save_lock_prevents_second_acquire() {
        let path = std::env::temp_dir().join(format!("v8q-save-lock-test-{}", std::process::id()));
        let lock = FileLock::acquire(&path, "busy").unwrap();
        assert!(FileLock::acquire(&path, "busy").is_err());
        drop(lock);
        assert!(FileLock::acquire(&path, "busy").is_ok());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn cleanable_buffer_files_exclude_logs() {
        assert!(is_cleanable_buffer_file(Path::new("/tmp/seg_000001.mkv")));
        assert!(is_cleanable_buffer_file(Path::new("/tmp/concat_test.txt")));
        assert!(is_cleanable_buffer_file(Path::new("/tmp/history.mkv")));
        assert!(!is_cleanable_buffer_file(Path::new("/tmp/ffmpeg.log")));
        assert!(!is_cleanable_buffer_file(Path::new(
            "/tmp/wl-screenrec.log"
        )));
    }

    #[test]
    fn list_clips_orders_by_modified_time() {
        let dir = std::env::temp_dir().join(format!("v8q-clips-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let first = dir.join("v8q_2026-01-01_00-00-01.mkv");
        let second = dir.join("v8q_2026-01-01_00-00-02.mkv");
        std::fs::write(&first, b"first").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        std::fs::write(&second, b"second").unwrap();
        std::fs::write(dir.join("other.mkv"), b"ignored").unwrap();

        let mut config = crate::Config::default();
        config.paths.output_dir = dir.to_string_lossy().into_owned();

        let clips = super::list_clips(&config).unwrap().clips;
        assert_eq!(clips, vec![first, second]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn wl_screenrec_failure_context_includes_actionable_debug_data() {
        let dir = std::env::temp_dir().join(format!("v8q-wls-context-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("probe.txt"), b"hello").unwrap();

        let mut config = crate::Config::default();
        config.paths.buffer_dir = dir.to_string_lossy().into_owned();
        let history = config.wl_screenrec_history_file();
        let context = wl_screenrec_failure_context(
            &config,
            std::process::id(),
            "wl-screenrec --history 30",
            &history,
            &["stale warning".to_string()],
        );

        assert!(context.contains("expected history file"));
        assert!(context.contains("buffer listing"));
        assert!(context.contains("probe.txt"));
        assert!(context.contains("log tail"));
        assert!(context.contains("beginner-safe"));
        assert!(context.contains("wl-screenrec-nvidia-compat"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn strips_basic_ansi_escape_sequences() {
        assert_eq!(strip_ansi("\u{1b}[31mERROR\u{1b}[0m plain"), "ERROR plain");
    }

    #[test]
    fn strips_csi_ansi_escape_sequences() {
        assert_eq!(strip_ansi("\u{1b}[2Kline\u{1b}[?25h"), "line");
    }

    #[test]
    fn finds_last_relevant_log_error() {
        let lines = ["ok", "ERROR first", "still ok", "Failed second"];
        assert_eq!(
            super::last_relevant_log_error(lines.iter().copied()),
            Some("Failed second".to_string())
        );
    }

    #[test]
    fn tiny_history_threshold_marks_zero_bytes_invalid() {
        let size = 0;
        assert!(size < super::MIN_VALID_HISTORY_BYTES);
    }
}

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{anyhow, Context};

use crate::config::{CaptureBackend, Config};
use crate::StartResult;
use crate::{paths, process};

pub fn start(config: &Config) -> anyhow::Result<StartResult> {
    let buffer_dir = config.paths.buffer_dir_path();
    let output_dir = config.paths.output_dir_path();
    let pid_file = config.paths.pid_file_path();

    process::ensure_not_running(&pid_file)?;
    std::fs::create_dir_all(&buffer_dir)
        .with_context(|| format!("failed to create buffer directory {}", buffer_dir.display()))?;
    std::fs::create_dir_all(&output_dir)
        .with_context(|| format!("failed to create output directory {}", output_dir.display()))?;
    paths::ensure_parent_dir(&pid_file)?;

    let args = build_record_command(config)?;
    let (program, program_args) = args
        .split_first()
        .ok_or_else(|| anyhow!("record command is empty"))?;

    let log_file_path = crate::paths::log_file_for_backend("ffmpeg");
    crate::paths::ensure_parent_dir(&log_file_path)?;
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
        .with_context(|| format!("failed to open {}", log_file_path.display()))?;
    let stderr = stdout
        .try_clone()
        .with_context(|| format!("failed to clone {}", log_file_path.display()))?;

    let child = Command::new(program)
        .args(program_args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .with_context(|| format!("failed to start record command: {}", args.join(" ")))?;

    let meta = process::new_meta(
        child.id(),
        config.effective_backend()?.as_str(),
        &args.join(" "),
        &buffer_dir,
        &output_dir,
        None,
        &Config::config_file_path()?,
        &log_file_path,
    );
    process::write_pid_with_meta(&pid_file, meta)?;
    Ok(StartResult {
        pid: child.id(),
        backend: config.effective_backend()?.as_str().to_string(),
        buffer_dir,
        log_file: log_file_path,
    })
}

pub fn build_record_command(config: &Config) -> anyhow::Result<Vec<String>> {
    match config.effective_backend()? {
        CaptureBackend::X11 => Ok(default_x11_command(config)),
        CaptureBackend::Custom => {
            if config.ffmpeg.custom_record_command.trim().is_empty() {
                return Err(anyhow!(
                    "capture backend 'custom' requires ffmpeg.custom_record_command"
                ));
            }

            let rendered = render_custom_command(&config.ffmpeg.custom_record_command, config);
            shell_words::split(&rendered).context("failed to parse custom_record_command")
        }
        CaptureBackend::WlScreenrec => Err(anyhow!(
            "wl-screenrec is not an FFmpeg segmented backend; use capture.start path"
        )),
    }
}

fn default_x11_command(config: &Config) -> Vec<String> {
    let r = &config.recording;
    let mut args = vec![
        "ffmpeg".to_string(),
        "-y".to_string(),
        "-f".to_string(),
        "x11grab".to_string(),
        "-framerate".to_string(),
        r.fps.to_string(),
        "-video_size".to_string(),
        format!("{}x{}", r.width, r.height),
        "-i".to_string(),
        ":0.0".to_string(),
        "-f".to_string(),
        "pulse".to_string(),
        "-i".to_string(),
        "default".to_string(),
        "-c:v".to_string(),
        r.encoder.clone(),
        "-preset".to_string(),
        "p5".to_string(),
        "-b:v".to_string(),
        r.video_bitrate.clone(),
        "-c:a".to_string(),
        r.audio_codec.clone(),
    ];

    args.extend(config.ffmpeg.extra_args.clone());
    args.extend(segment_args(config));
    args
}

fn segment_args(config: &Config) -> Vec<String> {
    let output_pattern = config
        .paths
        .buffer_dir_path()
        .join("seg_%06d.mkv")
        .to_string_lossy()
        .into_owned();

    vec![
        "-f".to_string(),
        "segment".to_string(),
        "-segment_time".to_string(),
        config.recording.segment_seconds.max(1).to_string(),
        "-segment_wrap".to_string(),
        config.ffmpeg_segment_wrap_count().to_string(),
        "-reset_timestamps".to_string(),
        "1".to_string(),
        output_pattern,
    ]
}

fn render_custom_command(template: &str, config: &Config) -> String {
    let r = &config.recording;
    let buffer_dir = config.paths.buffer_dir_path();
    let values = HashMap::from([
        ("fps", r.fps.to_string()),
        ("width", r.width.to_string()),
        ("height", r.height.to_string()),
        ("encoder", r.encoder.clone()),
        ("video_bitrate", r.video_bitrate.clone()),
        ("audio_codec", r.audio_codec.clone()),
        ("segment_seconds", r.segment_seconds.max(1).to_string()),
        ("buffer_dir", shell_quote_path(&buffer_dir)),
    ]);

    let mut rendered = template.to_string();
    for (key, value) in values {
        rendered = rendered.replace(&format!("{{{key}}}"), &value);
    }
    rendered
}

fn shell_quote_path(path: &Path) -> String {
    shell_words::quote(&path.to_string_lossy()).into_owned()
}

#[cfg(test)]
mod tests {
    use crate::config::Config;

    use super::build_record_command;

    #[test]
    fn x11_command_contains_segment_wrap() {
        let mut config = Config::default();
        config.capture.as_mut().unwrap().backend = "x11".to_string();

        let command = build_record_command(&config).unwrap();
        assert!(command.contains(&"-segment_wrap".to_string()));
        assert!(command.contains(&"18".to_string()));
    }

    #[test]
    fn custom_command_replaces_variables() {
        let mut config = Config::default();
        config.capture.as_mut().unwrap().backend = "custom".to_string();
        config.ffmpeg.custom_record_command =
            "ffmpeg -framerate {fps} -i test -segment_time {segment_seconds} {buffer_dir}/seg_%06d.mkv"
                .to_string();

        let command = build_record_command(&config).unwrap();
        assert_eq!(command[2], "60");
        assert!(command.iter().any(|arg| arg.contains("/seg_")));
    }
}

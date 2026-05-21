use crate::Config;

#[derive(Debug, Clone)]
pub struct Preset {
    pub name: &'static str,
    pub description: &'static str,
    pub audience: &'static str,
    pub risks: &'static str,
    pub fps: Option<u32>,
    pub encoder: Option<&'static str>,
    pub bitrate: Option<&'static str>,
    pub duration_seconds: Option<u64>,
    pub ffmpeg_encoder_options: Option<&'static str>,
    pub wl_screenrec_audio: Option<bool>,
    pub wl_screenrec_extra_args: Option<Vec<&'static str>>,
}

pub fn presets() -> Vec<Preset> {
    vec![
        Preset {
            name: "beginner-safe",
            description: "Most conservative first-run preset for Hyprland/NVIDIA troubleshooting.",
            audience: "New users who want the highest chance of getting the first clip saved.",
            risks: "Lower FPS, lower bitrate, no audio, and CPU encoding; quality is intentionally modest.",
            fps: Some(30),
            encoder: Some("libx264"),
            bitrate: Some("10M"),
            duration_seconds: Some(20),
            ffmpeg_encoder_options: Some(
                "preset=veryfast,tune=zerolatency,bf=0,g=30,x264-params=level=5.2",
            ),
            wl_screenrec_audio: Some(false),
            wl_screenrec_extra_args: Some(vec!["--experimental-vulkan"]),
        },
        Preset {
            name: "performance",
            description: "Good balance for an Nvidia gaming PC.",
            audience: "Most beginners with Nvidia GPUs.",
            risks: "Requires NVENC support in FFmpeg/wl-screenrec.",
            fps: Some(60),
            encoder: Some("h264_nvenc"),
            bitrate: Some("20M"),
            duration_seconds: Some(30),
            ffmpeg_encoder_options: None,
            wl_screenrec_audio: None,
            wl_screenrec_extra_args: None,
        },
        Preset {
            name: "quality",
            description: "Larger, cleaner clips.",
            audience: "Users who prefer quality over file size.",
            risks: "Higher bitrate and longer duration create larger files.",
            fps: Some(60),
            encoder: Some("h264_nvenc"),
            bitrate: Some("35M"),
            duration_seconds: Some(60),
            ffmpeg_encoder_options: None,
            wl_screenrec_audio: None,
            wl_screenrec_extra_args: None,
        },
        Preset {
            name: "low-end",
            description: "Lower impact for weaker PCs.",
            audience: "Older CPUs/GPUs or laptops.",
            risks: "Lower FPS and bitrate.",
            fps: Some(30),
            encoder: Some("h264_nvenc"),
            bitrate: Some("10M"),
            duration_seconds: Some(20),
            ffmpeg_encoder_options: None,
            wl_screenrec_audio: None,
            wl_screenrec_extra_args: None,
        },
        Preset {
            name: "cpu",
            description: "Software encoder fallback without NVENC.",
            audience: "Systems where NVENC is unavailable or broken.",
            risks: "Uses more CPU than hardware encoding.",
            fps: Some(30),
            encoder: Some("libx264"),
            bitrate: Some("12M"),
            duration_seconds: None,
            ffmpeg_encoder_options: None,
            wl_screenrec_audio: None,
            wl_screenrec_extra_args: None,
        },
        Preset {
            name: "nvidia",
            description: "Use Nvidia's H.264 encoder.",
            audience: "Nvidia users with working NVENC.",
            risks: "May fail if wl-screenrec cannot negotiate NVIDIA dmabuf formats.",
            fps: None,
            encoder: Some("h264_nvenc"),
            bitrate: None,
            duration_seconds: None,
            ffmpeg_encoder_options: Some("preset=p5"),
            wl_screenrec_audio: None,
            wl_screenrec_extra_args: None,
        },
        Preset {
            name: "no-audio",
            description: "Disable wl-screenrec audio capture.",
            audience: "Debugging setups where audio breaks recording.",
            risks: "Clips will not include audio.",
            fps: None,
            encoder: None,
            bitrate: None,
            duration_seconds: None,
            ffmpeg_encoder_options: None,
            wl_screenrec_audio: Some(false),
            wl_screenrec_extra_args: None,
        },
        Preset {
            name: "wl-screenrec-nvidia-compat",
            description: "Compatibility path for wl-screenrec on some NVIDIA/Hyprland systems.",
            audience:
                "Users seeing wl-screenrec format negotiation errors or history-mode x264 panics.",
            risks: "Uses CPU encoding and experimental Vulkan; treat as a workaround.",
            fps: Some(60),
            encoder: Some("libx264"),
            bitrate: Some("20M"),
            duration_seconds: None,
            ffmpeg_encoder_options: Some(
                "preset=veryfast,tune=zerolatency,bf=0,g=60,x264-params=level=5.2",
            ),
            wl_screenrec_audio: Some(false),
            wl_screenrec_extra_args: Some(vec!["--experimental-vulkan"]),
        },
    ]
}

pub fn explain(config: &Config, preset: &Preset) -> Vec<String> {
    let mut lines = vec![
        format!("Preset: {}", preset.name),
        format!("Description: {}", preset.description),
        format!("For: {}", preset.audience),
        format!("Risks: {}", preset.risks),
        "Changes:".to_string(),
    ];
    let diff = describe_diff(config, preset);
    if diff.is_empty() {
        lines.push("- no changes from current config".to_string());
    } else {
        lines.extend(diff.into_iter().map(|line| format!("- {line}")));
    }
    lines
}

pub fn find(name: &str) -> Option<Preset> {
    presets().into_iter().find(|preset| preset.name == name)
}

pub fn detect(config: &Config) -> Option<&'static str> {
    presets()
        .into_iter()
        .find(|preset| preset_matches(config, preset))
        .map(|preset| preset.name)
}

fn preset_matches(config: &Config, preset: &Preset) -> bool {
    preset.fps.is_none_or(|fps| config.recording.fps == fps)
        && preset
            .encoder
            .is_none_or(|encoder| config.wl_screenrec.ffmpeg_encoder == encoder)
        && preset
            .bitrate
            .is_none_or(|bitrate| config.wl_screenrec.bitrate == bitrate)
        && preset
            .duration_seconds
            .is_none_or(|duration| config.recording.duration_seconds == duration)
        && preset
            .ffmpeg_encoder_options
            .is_none_or(|options| config.wl_screenrec.ffmpeg_encoder_options == options)
        && preset
            .wl_screenrec_audio
            .is_none_or(|audio| config.wl_screenrec.audio == audio)
        && preset.wl_screenrec_extra_args.as_ref().is_none_or(|args| {
            config.wl_screenrec.extra_args
                == args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>()
        })
}

pub fn apply(config: &mut Config, preset: &Preset) {
    if let Some(fps) = preset.fps {
        config.recording.fps = fps;
    }
    if let Some(encoder) = preset.encoder {
        config.recording.encoder = encoder.to_string();
        config.wl_screenrec.ffmpeg_encoder = encoder.to_string();
    }
    if let Some(bitrate) = preset.bitrate {
        config.recording.video_bitrate = bitrate.to_string();
        config.wl_screenrec.bitrate = bitrate.to_string();
    }
    if let Some(duration) = preset.duration_seconds {
        config.recording.duration_seconds = duration;
    }
    if let Some(options) = preset.ffmpeg_encoder_options {
        config.wl_screenrec.ffmpeg_encoder_options = options.to_string();
    }
    if let Some(audio) = preset.wl_screenrec_audio {
        config.wl_screenrec.audio = audio;
    }
    if let Some(extra_args) = &preset.wl_screenrec_extra_args {
        config.wl_screenrec.extra_args = extra_args.iter().map(|arg| arg.to_string()).collect();
    }
}

pub fn describe_diff(config: &Config, preset: &Preset) -> Vec<String> {
    let mut changed = config.clone();
    apply(&mut changed, preset);
    let mut lines = Vec::new();
    if config.recording.fps != changed.recording.fps {
        lines.push(format!(
            "recording.fps: {} -> {}",
            config.recording.fps, changed.recording.fps
        ));
    }
    if config.recording.duration_seconds != changed.recording.duration_seconds {
        lines.push(format!(
            "recording.duration_seconds: {} -> {}",
            config.recording.duration_seconds, changed.recording.duration_seconds
        ));
    }
    if config.wl_screenrec.ffmpeg_encoder != changed.wl_screenrec.ffmpeg_encoder {
        lines.push(format!(
            "wl_screenrec.ffmpeg_encoder: {} -> {}",
            config.wl_screenrec.ffmpeg_encoder, changed.wl_screenrec.ffmpeg_encoder
        ));
    }
    if config.wl_screenrec.bitrate != changed.wl_screenrec.bitrate {
        lines.push(format!(
            "wl_screenrec.bitrate: {} -> {}",
            config.wl_screenrec.bitrate, changed.wl_screenrec.bitrate
        ));
    }
    if config.wl_screenrec.ffmpeg_encoder_options != changed.wl_screenrec.ffmpeg_encoder_options {
        lines.push(format!(
            "wl_screenrec.ffmpeg_encoder_options: {} -> {}",
            config.wl_screenrec.ffmpeg_encoder_options, changed.wl_screenrec.ffmpeg_encoder_options
        ));
    }
    if config.wl_screenrec.audio != changed.wl_screenrec.audio {
        lines.push(format!(
            "wl_screenrec.audio: {} -> {}",
            config.wl_screenrec.audio, changed.wl_screenrec.audio
        ));
    }
    if config.wl_screenrec.extra_args != changed.wl_screenrec.extra_args {
        lines.push(format!(
            "wl_screenrec.extra_args: {:?} -> {:?}",
            config.wl_screenrec.extra_args, changed.wl_screenrec.extra_args
        ));
    }
    lines
}

#[cfg(test)]
mod tests {
    #[test]
    fn detect_finds_applied_beginner_safe_preset() {
        let preset = super::find("beginner-safe").unwrap();
        let mut config = crate::Config::default();
        super::apply(&mut config, &preset);
        assert_eq!(super::detect(&config), Some("beginner-safe"));
    }

    #[test]
    fn dry_run_diff_reports_changes() {
        let preset = super::find("beginner-safe").unwrap();
        let config = crate::Config::default();
        let diff = super::describe_diff(&config, &preset);
        assert!(diff.iter().any(|line| line.contains("recording.fps")));
        assert!(diff.iter().any(|line| line.contains("wl_screenrec.audio")));
    }
}

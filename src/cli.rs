use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "v8q")]
#[command(about = "Small replay buffer recorder for Linux desktops")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Show a short first-run guide.
    Welcome,
    /// Start recording replay buffer segments.
    Start {
        /// Keep v8q alive for service managers.
        #[arg(long)]
        foreground: bool,
        /// Override wl-screenrec capture mode: output, geometry, or active-window.
        #[arg(long)]
        target: Option<String>,
        /// Override wl-screenrec output/monitor, for example DP-1.
        #[arg(long)]
        output: Option<String>,
        /// Override wl-screenrec geometry, for example "1366,0 1920x1080".
        #[arg(long)]
        geometry: Option<String>,
    },
    /// Stop the background FFmpeg recorder.
    Stop,
    /// Show recorder and buffer status.
    Status,
    /// Check local setup for V8Q, FFmpeg, wl-screenrec, PipeWire, and PATH.
    Doctor {
        /// Print machine-readable JSON.
        #[arg(long)]
        json: bool,
        /// Print detailed checks instead of beginner summary.
        #[arg(long)]
        verbose: bool,
    },
    /// Save the configured replay duration as a clip.
    Save {
        /// Append a sanitized name to the clip filename.
        #[arg(long)]
        name: Option<String>,
        /// Override replay duration for segmented backends.
        #[arg(long)]
        duration: Option<u64>,
        /// Open the saved clip.
        #[arg(long)]
        open: bool,
        /// Suppress notification.
        #[arg(long)]
        no_notify: bool,
        /// Print machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Remove buffer segments without deleting saved clips.
    Clean {
        #[command(subcommand)]
        command: Option<CleanCommand>,
    },
    /// Open the configured clips folder with xdg-open.
    OpenFolder,
    /// Show recent backend logs.
    Logs {
        /// Follow log output.
        #[arg(long)]
        follow: bool,
        /// Select backend log.
        #[arg(long)]
        backend: Option<String>,
        /// Number of lines to print.
        #[arg(long, default_value_t = 10)]
        lines: usize,
    },
    /// Config file helpers.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// List saved clips.
    Clips {
        /// Print only the latest clip.
        #[arg(long)]
        latest: bool,
        /// Open the latest clip with xdg-open.
        #[arg(long)]
        open_latest: bool,
        /// Delete the latest clip.
        #[arg(long)]
        delete_latest: bool,
        /// Print JSON.
        #[arg(long)]
        json: bool,
        /// Limit listed clips.
        #[arg(long)]
        limit: Option<usize>,
        /// Sort order.
        #[arg(long, value_enum, default_value_t = ClipSort::Oldest)]
        sort: ClipSort,
    },
    /// Manage one clip path.
    Clip {
        #[command(subcommand)]
        command: ClipCommand,
    },
    /// Setup helpers.
    Setup {
        #[command(subcommand)]
        command: Option<SetupCommand>,
    },
    /// Apply recording presets.
    Preset {
        #[command(subcommand)]
        command: PresetCommand,
    },
    /// Switch beginner/advanced output mode.
    Mode {
        #[command(subcommand)]
        command: ModeCommand,
    },
    /// List Hyprland windows that can be captured.
    Windows {
        #[arg(long)]
        json: bool,
    },
    /// Manage selected capture window.
    Window {
        #[command(subcommand)]
        command: WindowCommand,
    },
    /// User systemd service helpers.
    Service {
        #[command(subcommand)]
        command: ServiceCommand,
    },
    /// Focused debugging helpers.
    Debug {
        #[command(subcommand)]
        command: DebugCommand,
    },
    /// Audio helpers.
    Audio {
        #[command(subcommand)]
        command: AudioCommand,
    },
}

#[derive(Debug, Clone, ValueEnum)]
pub enum ClipSort {
    Newest,
    Oldest,
}

#[derive(Debug, Subcommand)]
pub enum CleanCommand {
    /// Clean log files.
    Logs {
        /// Remove logs older than duration like 7d.
        #[arg(long)]
        older_than: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Print the config path.
    Path,
    /// Print the resolved config.
    Show {
        /// Show resolved/effective config.
        #[arg(long)]
        resolved: bool,
    },
    /// Validate config.
    Validate,
    /// Open the config in $EDITOR, falling back to nano.
    Edit,
    /// Create the default config. Use --force to overwrite.
    Init {
        /// Overwrite an existing config file.
        #[arg(long)]
        force: bool,
    },
    /// Reset config to defaults, keeping a backup.
    Reset,
    /// Migrate legacy config to current shape.
    Migrate {
        /// Write the migrated config. Without this, only print what would happen.
        #[arg(long)]
        write: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum ClipCommand {
    Info {
        path: String,
    },
    Open {
        path: String,
    },
    Delete {
        path: String,
        #[arg(long)]
        force_external: bool,
    },
    Rename {
        path: String,
        new_name: String,
        #[arg(long)]
        overwrite: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum SetupCommand {
    Hyprland {
        #[arg(long)]
        write: bool,
    },
    Shell {
        #[arg(long)]
        write: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum PresetCommand {
    List,
    Explain {
        name: String,
    },
    Apply {
        name: String,
        #[arg(long)]
        write: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum ModeCommand {
    Beginner,
    Advanced,
    Show,
}

#[derive(Debug, Subcommand)]
pub enum WindowCommand {
    Select {
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        app_id: Option<String>,
        #[arg(long)]
        class: Option<String>,
        #[arg(long)]
        follow: bool,
    },
    Clear,
    Show,
}

#[derive(Debug, Subcommand)]
pub enum ServiceCommand {
    Install,
    Uninstall,
    Start,
    Stop,
    Status,
    Enable,
    Disable,
}

#[derive(Debug, Subcommand)]
pub enum DebugCommand {
    Info,
    WlScreenrec {
        /// Run wl-screenrec for N seconds, trigger history, and print the test log.
        #[arg(long)]
        test_run: Option<u64>,
    },
    Paths,
    Window,
    Audio,
}

#[derive(Debug, Subcommand)]
pub enum AudioCommand {
    Sources,
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{
        AudioCommand, Cli, Command, ConfigCommand, DebugCommand, ModeCommand, WindowCommand,
    };

    #[test]
    fn parses_new_top_level_commands() {
        assert!(matches!(
            Cli::try_parse_from(["v8q", "open-folder"]).unwrap().command,
            Command::OpenFolder
        ));
        assert!(matches!(
            Cli::try_parse_from(["v8q", "logs"]).unwrap().command,
            Command::Logs { .. }
        ));
    }

    #[test]
    fn parses_config_commands() {
        let cli = Cli::try_parse_from(["v8q", "config", "path"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Config {
                command: ConfigCommand::Path
            }
        ));
    }

    #[test]
    fn parses_clips_flags() {
        let cli = Cli::try_parse_from(["v8q", "clips", "--latest"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Clips {
                latest: true,
                open_latest: false,
                ..
            }
        ));
    }

    #[test]
    fn parses_debug_and_audio_commands() {
        let cli = Cli::try_parse_from(["v8q", "debug", "wl-screenrec", "--test-run", "5"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Debug {
                command: DebugCommand::WlScreenrec { test_run: Some(5) }
            }
        ));

        let cli = Cli::try_parse_from(["v8q", "audio", "sources"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Audio {
                command: AudioCommand::Sources
            }
        ));
    }

    #[test]
    fn parses_mode_and_window_commands() {
        let cli = Cli::try_parse_from(["v8q", "mode", "advanced"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Mode {
                command: ModeCommand::Advanced
            }
        ));

        let cli = Cli::try_parse_from(["v8q", "window", "select", "--title", "Firefox"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Window {
                command: WindowCommand::Select { .. }
            }
        ));
    }
}

# V8Q

A lightweight replay recorder for Linux/Hyprland, built around `wl-screenrec` and FFmpeg.

V8Q is a small CLI-first replay buffer recorder for Linux desktops. It targets Arch Linux/Omarchy with Hyprland/Wayland first, using `wl-screenrec --history` as the recommended backend, while keeping the replay/export core independent from the capture method.

Status: early but usable. Tested on Hyprland/NVIDIA, but Wayland setups vary.

It is intentionally not OBS, not Electron, not a streaming suite, and not a video editor.

Screenshots: coming soon.

## Features

- replay buffer with `v8q start`, `v8q save`, `v8q stop`
- Hyprland/Wayland backend via `wl-screenrec --history`
- FFmpeg/X11 and custom command fallback
- monitor capture, fixed geometry capture, and Hyprland window capture
- presets for beginner-friendly setup and debugging
- optional GTK/libadwaita GUI behind a feature flag
- doctor/debug commands for Linux desktop troubleshooting

## Install

Install dependencies on Arch:

```bash
sudo pacman -S ffmpeg pipewire wireplumber xdg-desktop-portal xdg-desktop-portal-hyprland
paru -S wl-screenrec
```

Build and install V8Q:

```bash
cargo build --release
cargo install --path .
```

The default install builds only the CLI. The GUI is optional and not installed by the default command.

If `v8q` is not found after install, `~/.cargo/bin` is probably missing from `PATH`.

For bash:

```bash
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

For zsh:

```bash
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

You can also run it directly:

```bash
~/.cargo/bin/v8q status
```

## Recommended First Run

For a first run on Hyprland/NVIDIA, start with video-only compatibility settings. This avoids audio and NVENC variables until the capture path is known to work:

```bash
v8q welcome
v8q doctor
v8q doctor --fix-plan
v8q preset apply beginner-safe --write
v8q debug wl-screenrec --test-run 5
```

Then run:

```bash
v8q doctor
v8q start
sleep 10
v8q save --name smoke
v8q stop
v8q clips --latest
```

Saved clips go to:

```text
~/Videos/V8Q
```

After video works, enable audio with a real monitor source:

```bash
v8q audio sources
```

Copy a `.monitor` source into `[wl_screenrec].audio_device`, then set `[wl_screenrec] audio = true`.

## If It Fails

If the smoke test fails, gather a report for an issue:

```bash
v8q debug report
v8q debug wl-screenrec --test-run 5
v8q doctor --verbose
```

On some NVIDIA/Hyprland systems, `wl-screenrec` can pass a short debug run and still crash during a longer history recording. V8Q reports that honestly with the backend log tail; it does not treat a stale PID as a successful recording.

Attach `v8q debug report` output when opening GitHub issues. It includes V8Q version, config/backend, session details, Hyprland details where available, wl-screenrec flags, FFmpeg encoder availability, service status, and recent logs.

## Basic Usage

```bash
v8q start
v8q save --name first-clip
v8q stop
```

Save with a readable suffix:

```bash
v8q save --name ace-1v3
v8q save --name ace-1v3 --open
v8q save --name ace-1v3 --reveal
v8q save --json
```

## Manual Test Matrix

Use this checklist before filing a bug or publishing a release:

```bash
# A) diagnostics
v8q doctor
v8q doctor --verbose
v8q status

# B) wl-screenrec backend probe
v8q debug wl-screenrec --test-run 5

# C) normal replay
v8q start
sleep 10
v8q save --name smoke-fixed
v8q stop
ffprobe -v error -show_entries format=duration,size -of json "$(v8q clips --latest)"

# D) NVIDIA/Hyprland compatibility preset
v8q preset apply wl-screenrec-nvidia-compat --write
v8q start
sleep 10
v8q save --name nvidia-compat-test
v8q stop

# E) Hyprland window capture
v8q windows
v8q window select
v8q window show
v8q start
sleep 10
v8q save --name window-test
v8q stop
v8q window clear
```

Expected result: `save` prints `Saved replay: ...`, `ffprobe` can read the file, and `v8q status` does not report a stale recorder PID.

## GUI

V8Q has an initial native GTK4/libadwaita GUI behind the optional `gui` feature. The CLI remains the core product path; the GUI calls the same public Rust library functions as the CLI and does not duplicate recorder logic.

Install GUI development/runtime packages on Arch:

```bash
sudo pacman -S gtk4 libadwaita
```

Build the GUI:

```bash
cargo build --features gui --bin v8q-gui
```

Run the GUI:

```bash
cargo run --features gui --bin v8q-gui
```

The current GUI is intentionally small. It shows recorder status, PID, backend, replay duration, FPS, encoder, bitrate, buffer/output paths, latest clip, and feedback messages. It has buttons for Start, Save Replay, Stop, Refresh, Open Clips Folder, Doctor, and Settings.

GUI TODO before calling it polished: show the latest 5 clips, expose capture target/window selection, add select/clear window controls, and make error details easier to copy from the window.

Long-running GUI actions run on worker threads with `std::thread`, then update GTK on the main loop. No async runtime is used.

## Commands

```bash
v8q welcome
v8q setup
v8q start
v8q stop
v8q status
v8q status --json
v8q doctor
v8q doctor --fix-plan
v8q debug report
v8q save
v8q clean
v8q logs
v8q logs --tail 50
v8q logs --clear
v8q open-folder
v8q config path
v8q config show
v8q config show --json
v8q config edit
v8q config init --force
v8q clips
v8q clips --latest
v8q clips --open-latest
v8q clip info "$(v8q clips --latest)"
v8q clip open "$(v8q clips --latest)"
v8q clip reveal "$(v8q clips --latest)"
v8q setup
v8q setup hyprland
v8q setup shell
v8q preset list
v8q preset apply beginner-safe --write
v8q preset explain performance
v8q preset apply nvidia --write
v8q preset apply wl-screenrec-nvidia-compat --write
v8q mode beginner
v8q mode advanced
v8q windows
v8q window select --title Firefox
v8q window select --interactive
v8q window show
v8q debug info
v8q debug window
v8q service print
v8q service install
v8q service enable
v8q service start
```

`v8q doctor` checks PATH, FFmpeg, `wl-screenrec`, NVENC encoder availability, PipeWire/portal user services, config loading, backend recognition, and configured paths. It exits with code `1` only when it finds a failing check.

`v8q doctor --fix-plan` does not change the system. It prints the commands V8Q recommends for the current machine, such as adding Cargo to `PATH`, installing missing packages, applying `beginner-safe`, or running the wl-screenrec debug probe.

## Configuration

V8Q reads:

```text
~/.config/v8q/config.toml
```

If the file does not exist, V8Q creates a default config:

```toml
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
enabled = false
title = ""
class = ""
address = ""
geometry = ""
follow = false

[ffmpeg]
custom_record_command = ""
extra_args = []

[wl_screenrec]
capture_mode = "output"
auto_select_focused_output = true
output = ""
geometry = ""
audio = true
audio_device = "default"
audio_backend = "pulse"
ffmpeg_encoder = "h264_nvenc"
ffmpeg_encoder_options = "preset=p5"
bitrate = "20M"
extra_args = []

[ui]
start_minimized = false
close_to_tray = false
show_notifications = true
theme = "system"
mode = "beginner"

[notifications]
enabled = true
command = "notify-send"
on_save = true
on_error = true
on_start = false
on_stop = false
```

V8Q still accepts the old config format:

```toml
[ffmpeg]
capture_backend = "hyprland"
```

If `[capture].backend` exists, it wins. If not, V8Q falls back to old `[ffmpeg].capture_backend`.

Supported backends:

- `wl-screenrec`, `hyprland`, `wayland`: use the `wl-screenrec` backend
- `x11`: use FFmpeg with `x11grab`
- `custom`: use `ffmpeg.custom_record_command`

## Hyprland Backend

`wl-screenrec` is the recommended backend for Hyprland because it already handles wlroots/Wayland capture efficiently and can use FFmpeg encoders such as `h264_nvenc`.

`v8q start` runs a command shaped like:

```bash
wl-screenrec \
  --history 30 \
  --filename /tmp/v8q-buffer/history.mkv \
  --ffmpeg-encoder h264_nvenc \
  --audio-codec aac \
  --bitrate "20 MB" \
  --audio \
  --audio-device default \
  --audio-backend pulse \
  --ffmpeg-encoder-options preset=p5 \
  --max-fps 60
```

Before starting, V8Q checks that `wl-screenrec` exists and that `wl-screenrec --help` exposes the critical flags `--history` and `--filename`. Other flags are only passed when the local `wl-screenrec --help` says they exist, including `--max-fps`, encoder, bitrate, and audio flags. `v8q doctor` reports missing optional flags as warnings.

`v8q save` sends `SIGUSR1` to flush the history buffer, waits for the history file, terminates the recorder to finalize the MKV, moves the clip to `~/Videos/V8Q`, and restarts the recorder. A simple lock under the V8Q runtime directory prevents simultaneous saves.

`v8q clean` removes buffer segments and temporary history/concat files, but keeps `ffmpeg.log`, `wl-screenrec.log`, and saved clips.

## Audio

List PipeWire/Pulse sources:

```bash
v8q audio sources
```

For desktop audio, look for a `.monitor` source and set:

```toml
[wl_screenrec]
audio = true
audio_device = "alsa_output.pci-0000_00_1f.3.analog-stereo.monitor"
```

To disable audio:

```toml
[wl_screenrec]
audio = false
```

Or apply the quick preset:

```bash
v8q preset apply no-audio --write
```

## Monitor Selection

To record a specific Hyprland output:

```toml
[wl_screenrec]
output = "DP-1"
```

If `output` and `geometry` are empty, V8Q auto-selects the focused Hyprland monitor with `hyprctl monitors -j`. On a dual-monitor setup this avoids the `multiple enabled displays and no --geometry or --output supplied` wl-screenrec error.

To record only the currently focused program/window, use the active-window target:

```toml
[wl_screenrec]
capture_mode = "active-window"
```

Or run it just for this start:

```bash
v8q start --target active-window
```

To persistently select a program/window:

```bash
v8q windows
v8q windows --json
v8q window select --title Firefox
v8q window select --class firefox
v8q window select --app-id steam
v8q window select --interactive
v8q start
v8q save --name firefox
```

This uses `hyprctl activewindow -j` and passes the window rectangle to `wl-screenrec --geometry`. It captures the window's current rectangle; if you move or resize the window after starting, restart V8Q.

To record a fixed region:

```toml
[wl_screenrec]
capture_mode = "geometry"
geometry = "1366,0 1920x1080"
```

You can also override the monitor for one run:

```bash
v8q start --output DP-1
```

Use `hyprctl monitors` to inspect monitor names.

## Hyprland Binds

Put binds in your Hyprland config after installing V8Q somewhere in `PATH`:

```conf
bind = SUPER_SHIFT, R, exec, v8q save
bind = SUPER_SHIFT, S, exec, v8q start
bind = SUPER_SHIFT, X, exec, v8q stop
```

Hotkeys stay outside the app for now. Use Hyprland binds such as:

```conf
bind = SUPER_SHIFT, R, exec, v8q save
```

On Omarchy, Hyprland config usually lives under `~/.config/hypr/`. Run `hyprctl reload` after editing if it does not auto-reload.

## Logs And Clips

Show recent backend logs:

```bash
v8q logs
v8q logs --tail 50
v8q logs --clear
```

Open the clips folder:

```bash
v8q open-folder
```

List saved clips or work with the latest clip:

```bash
v8q clips
v8q clips --latest
v8q clips --open-latest
v8q clip info "$(v8q clips --latest)"
v8q clip reveal "$(v8q clips --latest)"
```

## Config Commands

```bash
v8q config path
v8q config show
v8q config show --json
v8q config show --resolved
v8q config validate
v8q config edit
v8q config init
v8q config init --force
v8q config migrate
v8q config migrate --write
v8q config reset
```

`config edit` uses `$EDITOR` and falls back to `nano`. `config migrate` is a dry run; add `--write` to create a backup and update legacy `[ffmpeg].capture_backend`, `/tmp/v8q-buffer`, and `/tmp/v8q.pid` defaults.

## Setup

```bash
v8q setup
v8q setup shell
v8q setup shell --write
v8q setup hyprland
v8q setup hyprland --write
```

Setup creates configured directories, validates config, and prints PATH and Hyprland bind instructions. It does not start recording automatically. `--write` is required before V8Q appends to shell or Hyprland config files.

## Presets

```bash
v8q preset list
v8q preset apply beginner-safe --write
v8q preset apply performance
v8q preset apply quality
v8q preset apply low-end
v8q preset apply cpu
v8q preset apply nvidia --write
```

Without `--write`, preset apply prints what would change.

Recommended starting points:

- `beginner-safe`: first-run preset, 30 FPS, modest bitrate, no audio, `libx264`, B-frames disabled.
- `wl-screenrec-nvidia-compat`: keeps 60 FPS but uses the same low-latency x264 history settings for NVIDIA/Hyprland troubleshooting.
- `performance`: NVENC path for systems where `wl-screenrec` and the driver cooperate cleanly.

## Service

```bash
v8q service print
v8q service install
v8q service enable
v8q service start
v8q service status
```

The user service is written to `~/.config/systemd/user/v8q.service` and uses `v8q start --foreground`.

## X11 Fallback

X11 remains available:

```toml
[capture]
backend = "x11"
```

The generated FFmpeg command uses `x11grab`, Pulse audio, NVENC by default, and FFmpeg segment wrapping:

```bash
ffmpeg -y \
  -f x11grab \
  -framerate 60 \
  -video_size 1920x1080 \
  -i :0.0 \
  -f pulse \
  -i default \
  -c:v h264_nvenc \
  -preset p5 \
  -b:v 20M \
  -c:a aac \
  -f segment \
  -segment_time 2 \
  -segment_wrap 18 \
  -reset_timestamps 1 \
  /tmp/v8q-buffer/seg_%06d.mkv
```

## Custom FFmpeg Backend

Use this when you want to own the full FFmpeg capture command:

```toml
[capture]
backend = "custom"

[ffmpeg]
custom_record_command = "ffmpeg -y -f x11grab -framerate {fps} -video_size {width}x{height} -i :0.0 -f pulse -i default -c:v {encoder} -preset p5 -b:v {video_bitrate} -c:a {audio_codec} -f segment -segment_time {segment_seconds} -segment_wrap 18 -reset_timestamps 1 {buffer_dir}/seg_%06d.mkv"
extra_args = []
```

Supported variables:

- `{fps}`
- `{width}`
- `{height}`
- `{encoder}`
- `{video_bitrate}`
- `{audio_codec}`
- `{segment_seconds}`
- `{buffer_dir}`

## V8Q vs gpu-screen-recorder

V8Q is not claiming to beat `gpu-screen-recorder`.

`gpu-screen-recorder` may be more mature, more performance-focused, and a better fit if you want a recorder centered on GPU capture efficiency today. V8Q is a smaller CLI-first project focused on:

- simple replay-buffer workflow with `start`, `save`, and `stop`
- readable TOML config and presets
- `doctor` and `debug` commands for Hyprland/Linux troubleshooting
- Hyprland monitor/window helpers
- backend-independent clip/export management
- optional native GTK/libadwaita GUI

A future `gpu-screen-recorder` backend is under consideration. See [docs/backends.md](docs/backends.md).

## File Tree

```text
.
├── Cargo.toml
├── README.md
├── config.example.toml
└── src
    ├── bin
    │   └── v8q-gui.rs
    ├── cli.rs
    ├── config.rs
    ├── doctor.rs
    ├── error.rs
    ├── ffmpeg.rs
    ├── lib.rs
    ├── lock.rs
    ├── main.rs
    ├── paths.rs
    ├── process.rs
    ├── replay.rs
    └── wl_screenrec.rs
```

## Modules

- `lib.rs`: public API shared by CLI and future GUI
- `main.rs`: parses CLI arguments and prints structured results from the library
- `cli.rs`: Clap command definitions
- `config.rs`: TOML config, defaults, backend resolution, and compatibility with old config
- `wl_screenrec.rs`: starts the `wl-screenrec --history` backend
- `doctor.rs`: local environment checks for Arch/Hyprland setup
- `ffmpeg.rs`: FFmpeg command construction and segmented recorder startup
- `lock.rs`: small lock-file helper
- `process.rs`: PID file, process checks, signals, and PID metadata
- `replay.rs`: status, save, segment listing, history export, log tails, and cleanup
- `paths.rs`: config path lookup, directory creation, and `~` expansion
- `error.rs`: shared result alias

## GUI Architecture

The CLI core has been split into a Rust library so a graphical UI can call the same functions:

- `start_recorder`
- `stop_recorder`
- `save_replay`
- `get_status`
- `run_doctor`
- `clean_buffer`

The planned GUI toolkit is GTK4 + libadwaita because it is native on Linux, works well on Wayland, fits Hyprland/Arch better than a browser shell, and is enough for status, buttons, settings, and diagnostics. V8Q intentionally does not use Electron, Tauri, or a webview for this UI.

Build the GUI with:

```bash
cargo build --features gui --bin v8q-gui
```

If GTK4/libadwaita development packages are not installed, that build can fail at the system dependency step. The default CLI build does not require GTK.

## Troubleshooting

Run:

```bash
v8q doctor
```

Common issues:

- `v8q: command not found`: add `~/.cargo/bin` to `PATH` or run `~/.cargo/bin/v8q status`.
- `wl-screenrec not found`: install it with `paru -S wl-screenrec`.
- `h264_nvenc` missing: confirm NVIDIA drivers and FFmpeg NVENC support with `ffmpeg -hide_banner -encoders | grep nvenc`.
- no desktop audio: run `pactl list short sources`, find a `.monitor` source, and set `wl_screenrec.audio_device`.
- portal inactive: check `systemctl --user status xdg-desktop-portal xdg-desktop-portal-hyprland`.

Focused wl-screenrec debugging:

```bash
v8q debug wl-screenrec
v8q debug wl-screenrec --test-run 5
```

Manual equivalent test:

```bash
mkdir -p /tmp/v8q-buffer
rm -f /tmp/v8q-buffer/history.mkv

wl-screenrec \
  --history 30 \
  --filename /tmp/v8q-buffer/history.mkv \
  --ffmpeg-encoder h264_nvenc \
  --ffmpeg-encoder-options preset=p5 \
  --max-fps 60
```

In another terminal:

```bash
pkill -USR1 wl-screenrec
ls -lh /tmp/v8q-buffer/history.mkv
```

If that works without audio but V8Q fails with audio enabled, run `v8q audio sources`, choose a `.monitor` source for `audio_device`, or test with `[wl_screenrec] audio = false`.

On some NVIDIA/Hyprland setups `wl-screenrec` can fail before recording with `Failed to negotiate format`. The compatibility preset uses the path that is most likely to work with current wl-screenrec on those systems:

```bash
v8q preset apply wl-screenrec-nvidia-compat --write
```

It switches the wl-screenrec encoder to `libx264`, disables audio for the first test, adds `--experimental-vulkan`, and configures x264 for low-latency history recording with B-frames disabled. That matters because some wl-screenrec versions can panic in history mode when encoded packets arrive with reordered PTS. After confirming video works, re-enable audio with a real `.monitor` source from `v8q audio sources`.

Known current limitation: `wl-screenrec --history` can still panic on some NVIDIA/Vulkan combinations during longer captures. If `v8q debug wl-screenrec --test-run 5` works but `v8q start; sleep 10; v8q save` fails, keep the log from `v8q status` or `v8q logs --backend wl-screenrec` and test a newer `wl-screenrec`, a different encoder/preset, or the X11/custom backend. This is a backend stability issue, not the GUI.

## Future Improvements

- add a native PipeWire capture backend
- add tested `wf-recorder` fallback templates
- probe actual clip duration instead of segment-count approximation
- add package metadata for Arch installation

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Keep changes small, Linux-focused, and easy to debug.

## License

MIT. See [LICENSE](LICENSE).

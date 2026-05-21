# V8Q

A lightweight replay recorder for Linux/Hyprland, built around `wl-screenrec` and FFmpeg.

Status: early but usable. V8Q is Linux-only, best tested on Hyprland/Wayland, and intentionally not OBS, Electron, a streaming suite, or a video editor.

## Features

- replay buffer: `v8q start`, `v8q save`, `v8q stop`
- Hyprland/Wayland backend via `wl-screenrec --history`
- FFmpeg/X11 and custom command fallback
- Hyprland monitor/window helpers
- presets for first-run and NVIDIA/Hyprland troubleshooting
- `doctor` and `debug report` diagnostics for GitHub issues
- optional experimental GTK/libadwaita GUI

## Install

Arch/Omarchy dependencies:

```bash
sudo pacman -S ffmpeg pipewire wireplumber xdg-desktop-portal xdg-desktop-portal-hyprland
paru -S wl-screenrec
```

Build and install:

```bash
cargo install --path .
```

If `v8q` is not found:

```bash
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

## Quickstart

For a first run on Hyprland/NVIDIA, test video without audio first:

```bash
v8q doctor
v8q preset apply beginner-safe --write
v8q debug wl-screenrec --test-run 5
v8q start
sleep 10
v8q save --name smoke
v8q stop
v8q clips --latest
```

Clips are saved to:

```text
~/Videos/V8Q
```

After video works, enable audio with a real monitor source from:

```bash
v8q audio sources
```

## Basic Usage

```bash
v8q start
v8q save --name clutch-1v3
v8q stop
v8q clips
v8q clip info "$(v8q clips --latest)"
```

Useful diagnostics:

```bash
v8q status
v8q logs --lines 50
v8q doctor --verbose
v8q debug report
```

## Hyprland Binds

Add binds to `~/.config/hypr/hyprland.conf`:

```conf
bind = SUPER_SHIFT, R, exec, v8q save
bind = SUPER_SHIFT, S, exec, v8q start
bind = SUPER_SHIFT, X, exec, v8q stop
bind = SUPER_SHIFT, D, exec, v8q doctor
```

Hotkeys stay outside V8Q; Hyprland owns global keybinds.

## Window Capture

```bash
v8q windows
v8q window select --interactive
v8q window show
v8q start
v8q save --name window-test
v8q stop
```

Window capture is Hyprland-only and depends on `hyprctl` plus `wl-screenrec --geometry`.

## V8Q vs gpu-screen-recorder

V8Q is not claiming to beat `gpu-screen-recorder`.

`gpu-screen-recorder` may be more mature and more performance-focused, especially if your main goal is efficient GPU recording today. V8Q focuses on a small CLI workflow, readable config, presets, doctor/debug output, Hyprland window helpers, clip management, and an optional native GUI. A future `gpu-screen-recorder` backend is under consideration.

See [docs/backends.md](docs/backends.md).

## Known Limitations

- Linux only.
- Best tested on Hyprland/Wayland.
- `wl-screenrec` behavior varies by compositor, driver, and GPU.
- NVIDIA/Hyprland/Vulkan setups can still expose `wl-screenrec` instability.
- GUI is optional and experimental.
- V8Q is a replay recorder, not an OBS replacement.

## Docs

- [Getting started](docs/getting-started.md)
- [Hyprland setup](docs/hyprland.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Backends and roadmap](docs/backends.md)
- [Configuration](docs/config.md)
- [Window capture](docs/window-capture.md)
- [GUI](docs/gui.md)
- [Release checklist](docs/release-checklist.md)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Keep changes small, Linux-focused, and easy to debug.

## License

MIT. See [LICENSE](LICENSE).

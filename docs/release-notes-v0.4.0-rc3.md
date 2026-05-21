# V8Q v0.4.0-rc3

Early release candidate for V8Q, a lightweight replay recorder for Linux/Hyprland built around `wl-screenrec` and FFmpeg.

## Summary

This release candidate focuses on making V8Q easier to try, diagnose, and report issues for. The core workflow remains CLI-first:

```bash
v8q start
v8q save
v8q stop
```

## Highlights

- Hyprland/Wayland replay buffer through `wl-screenrec --history`.
- Conservative `beginner-safe` preset for first-run testing.
- `v8q doctor`, `v8q doctor --fix-plan`, and `v8q debug report`.
- Cleaner `v8q status` and `v8q logs` output.
- Better `v8q clip info` diagnostics using `ffprobe`.
- Hyprland window helpers: `v8q windows`, `v8q window select`, `v8q window show`.
- Optional experimental GTK/libadwaita GUI.
- FFmpeg/X11 and custom command fallbacks remain available.

## Install

Arch/Omarchy dependencies:

```bash
sudo pacman -S ffmpeg pipewire wireplumber xdg-desktop-portal xdg-desktop-portal-hyprland
paru -S wl-screenrec
```

Build/install from source:

```bash
cargo install --path .
```

If `v8q` is not found:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

## Recommended Test

```bash
v8q doctor
v8q preset apply beginner-safe --write
v8q debug wl-screenrec --test-run 5
v8q start
sleep 10
v8q save --name rc3-smoke --json
v8q stop
v8q clip info "$(v8q clips --latest)"
```

## Known Issues

- Linux only.
- Best tested on Hyprland/Wayland.
- `wl-screenrec` behavior varies by compositor, GPU, and driver.
- Some NVIDIA/Hyprland/Vulkan setups can still hit `wl-screenrec` instability during longer history recordings.
- Start with `beginner-safe` video-only settings, then enable audio after video capture works.
- The GUI is experimental.
- V8Q is not OBS and is not trying to replace OBS.

## Note on gpu-screen-recorder

V8Q is not claiming to outperform `gpu-screen-recorder`. `gpu-screen-recorder` may be more mature and performance-focused. V8Q currently focuses on a small CLI workflow, config/presets, doctor/debug output, Hyprland helpers, and clip management. A future `gpu-screen-recorder` backend is under consideration.

## Reporting Issues

Please include:

```bash
v8q debug report
v8q status
v8q logs --lines 100
v8q debug wl-screenrec --test-run 5
```

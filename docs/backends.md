# Backends

V8Q keeps capture backends separate from the CLI workflow, config, status, clip management, and diagnostics. The project is not trying to become OBS. The goal is a small replay recorder that can swap capture implementations when that is useful.

## wl-screenrec

Recommended for Hyprland/Wayland. V8Q uses `wl-screenrec --history` and only passes optional flags when the local `wl-screenrec --help` lists them.

Current status:

- default Wayland/Hyprland backend
- supports output capture and geometry/window capture when local `wl-screenrec` supports the needed flags
- can be unstable on some NVIDIA/Vulkan setups during longer history recordings
- V8Q detects stale PID/crashes and preserves logs instead of pretending recording worked

## gpu-screen-recorder candidate

`gpu-screen-recorder` is a serious candidate for a future backend. It may be more mature and performance-focused than V8Q's current `wl-screenrec` path.

Why consider it:

- designed around efficient GPU recording
- likely attractive for NVIDIA/AMD gaming replay use cases
- may reduce the wl-screenrec/NVIDIA/Vulkan instability some users hit

Open questions:

- how best to integrate replay/history behavior
- clip save signaling/API behavior
- audio handling and monitor/window target support
- packaging expectations across Arch/Omarchy and other distros

V8Q should not wrap it blindly. A backend should keep the current CLI behavior: `v8q start`, `v8q save`, `v8q stop`, `v8q status`, logs, doctor, and config.

## wf-recorder fallback candidate

`wf-recorder` is a possible wlroots fallback. It may be useful where `wl-screenrec` is not available or not stable.

Open questions:

- replay-buffer/history support may need segment-based recording rather than native history
- encoder/audio options vary from the current wl-screenrec flow
- window/geometry capture behavior must be validated

## FFmpeg/X11

Uses FFmpeg `x11grab` and segment muxing.

This is kept for X11 sessions and development fallback. It is not the preferred Hyprland/Wayland path.

## custom FFmpeg

Uses `ffmpeg.custom_record_command`. This is powerful but risky: review command strings carefully and avoid shell-style concatenation.

Use this only when you understand the capture command you want. V8Q can still manage PID/status/logs/clips around the custom command, but it cannot make arbitrary commands safe or portable.

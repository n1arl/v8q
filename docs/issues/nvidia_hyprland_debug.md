# NVIDIA / Hyprland Debug Report

Use this when `wl-screenrec` crashes, the history file is tiny, or saving times out.

## Required commands

```bash
v8q doctor --verbose
v8q status
v8q debug wl-screenrec --test-run 5
v8q logs --backend wl-screenrec --tail 100
```

## Smoke test

```bash
v8q preset apply beginner-safe --write
v8q start
sleep 10
v8q save --name debug
v8q stop
```

## System

- Distro:
- Kernel:
- GPU:
- NVIDIA driver version:
- Hyprland version:
- wl-screenrec version:
- Monitor/output name, for example DP-1:

## Notes

Mention whether `beginner-safe` or `wl-screenrec-nvidia-compat` changes the behavior.

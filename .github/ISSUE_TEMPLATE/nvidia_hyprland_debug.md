---
name: NVIDIA / Hyprland debug
about: Report wl-screenrec or NVIDIA/Hyprland capture problems
title: ""
labels: backend, hyprland, nvidia
assignees: ""
---

## Required commands

```bash
v8q doctor --verbose
v8q status
v8q debug wl-screenrec --test-run 5
v8q logs --backend wl-screenrec --tail 100
```

## Smoke test result

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
- NVIDIA driver:
- Hyprland version:
- wl-screenrec version:
- Output name:

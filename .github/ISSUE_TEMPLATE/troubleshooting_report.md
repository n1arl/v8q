---
name: Troubleshooting report
about: Share diagnostics for start/save/backend failures
title: ""
labels: troubleshooting
assignees: ""
---

## Problem

## Required output

```bash
v8q debug report
v8q status
v8q debug wl-screenrec --test-run 5
v8q logs --tail 100
```

## Smoke test

```bash
v8q preset apply beginner-safe --write
v8q start
sleep 10
v8q save --name issue-smoke --json
v8q stop
```

## System

- Distro:
- Compositor:
- GPU:
- Driver:
- Monitors:

## Notes

Does video-only work? Does audio change the behavior?

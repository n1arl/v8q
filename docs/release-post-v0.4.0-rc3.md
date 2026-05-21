# Short Release Post

I built V8Q, a small CLI-first replay recorder for Linux/Hyprland.

It is early, but usable on my Hyprland/NVIDIA setup. The basic workflow is:

```bash
v8q start
v8q save
v8q stop
```

It uses `wl-screenrec --history` today, has presets, `doctor`/`debug report` commands, Hyprland window helpers, clip management, and an optional experimental GTK GUI.

This is not meant to be OBS, and I am not claiming it beats `gpu-screen-recorder`. If your priority is the most mature/performance-focused GPU recorder, `gpu-screen-recorder` may be the better option right now. V8Q is more about a simple CLI replay workflow, readable config, troubleshooting output, and eventually being able to swap capture backends. A `gpu-screen-recorder` backend is on the roadmap to investigate.

The rough edge: `wl-screenrec` can still be unstable on some NVIDIA/Hyprland/Vulkan combinations, so the README recommends starting with a conservative video-only preset and enabling audio after video works.

Repo: https://github.com/n1arl/v8q

Useful first test:

```bash
v8q doctor
v8q preset apply beginner-safe --write
v8q debug wl-screenrec --test-run 5
v8q start
sleep 10
v8q save --name smoke
v8q stop
```

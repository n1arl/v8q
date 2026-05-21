# Changelog

## 0.4.0 - Unreleased

- Improved first-run onboarding with `welcome`, interactive `setup`, and `doctor --fix-plan`.
- Improved `status` output and added `v8q status --json`.
- Improved save UX with `--reveal`, structured JSON, unique output names, and clearer backend failure guidance.
- Improved clip management with richer listings, `ffprobe`-backed `clip info`, and `clip reveal`.
- Improved window capture UX with `window select --interactive`.
- Improved logs with `--tail` alias and `--clear`.
- Improved service management with `v8q service print`.
- Improved presets and beginner guidance around `beginner-safe`.
- Improved optional GUI status with capture target and recent errors.
- Detached `wl-screenrec` into its own process group so `v8q start` behaves like the debug test-run after the CLI exits.
- Added `v8q debug report` for GitHub issue diagnostics.
- Added honest V8Q vs `gpu-screen-recorder` documentation and backend roadmap notes.
- Added GitHub issue/debug templates.

## 0.3.0 - Unreleased

- Added `wl-screenrec --history` backend for Hyprland/Wayland.
- Added FFmpeg/X11 and custom command backend support.
- Added `doctor` and `debug wl-screenrec` diagnostics.
- Added presets, including `beginner-safe` and `wl-screenrec-nvidia-compat`.
- Added Hyprland window listing and window/geometry capture support.
- Added setup, service, clips, config, logs, mode, and save commands.
- Switched defaults toward XDG runtime/state paths.
- Added safer PID metadata, stale PID cleanup, and lock metadata.
- Improved status/history validation and save error messages.
- Added release-readiness smoke testing notes.
- Added initial GTK/libadwaita GUI behind the optional `gui` feature.
- Added GitHub docs, CI, MIT license, and local install script.

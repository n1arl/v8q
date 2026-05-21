# Backends

## wl-screenrec

Recommended for Hyprland/Wayland. V8Q uses `wl-screenrec --history` and only passes optional flags when the local `wl-screenrec --help` lists them.

## X11

Uses FFmpeg `x11grab` and segment muxing.

## custom

Uses `ffmpeg.custom_record_command`. This is powerful but risky: review command strings carefully and avoid shell-style concatenation.


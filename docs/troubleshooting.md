# Troubleshooting

Start with:

```bash
v8q doctor
v8q doctor --fix-plan
v8q preset apply beginner-safe --write
```

Common fixes:

- Add Cargo bin to PATH: `v8q setup shell`
- Check machine-readable state: `v8q status --json`
- Install wl-screenrec: `paru -S wl-screenrec`
- Check PipeWire services: `systemctl --user status pipewire wireplumber`
- Check portals: `systemctl --user status xdg-desktop-portal xdg-desktop-portal-hyprland`
- Check NVENC: `ffmpeg -hide_banner -encoders | grep nvenc`

For wl-screenrec history/save failures:

```bash
v8q debug wl-screenrec
v8q debug wl-screenrec --test-run 5
```

Manual no-audio probe:

```bash
mkdir -p /tmp/v8q-buffer
rm -f /tmp/v8q-buffer/history.mkv
wl-screenrec --history 30 --filename /tmp/v8q-buffer/history.mkv --ffmpeg-encoder h264_nvenc --ffmpeg-encoder-options preset=p5 --max-fps 60
```

Then run in another terminal:

```bash
pkill -USR1 wl-screenrec
ls -lh /tmp/v8q-buffer/history.mkv
```

If audio causes failures, run `v8q audio sources` and copy a `.monitor` source into `[wl_screenrec].audio_device`, or test with `v8q preset apply no-audio --write`.

If `v8q status` says the history file exists but is too small, it usually means the recorder crashed or has not flushed a real replay yet. Check `v8q logs --tail 50 --backend wl-screenrec` and run the debug probe.

If `v8q debug wl-screenrec --test-run 5` creates a valid file but a longer smoke test fails with a `wl-screenrec` panic, keep the `v8q status` or `v8q logs --backend wl-screenrec` output. Some wl-screenrec history builds panic when x264 emits reordered packets with B-frames. Run `v8q preset apply beginner-safe --write` first, then `v8q preset apply wl-screenrec-nvidia-compat --write` if you want to try 60 FPS. Both use `libx264` with low-latency options and B-frames disabled. If it still fails, try a newer `wl-screenrec`, lower FPS/bitrate, or a different backend before assuming the GUI is involved.

If `debug wl-screenrec --test-run` works but `v8q start` used to die shortly after the CLI exited, v0.4.0 starts `wl-screenrec` in its own Unix process group. Re-test with `v8q start; sleep 10; v8q save --name smoke`.

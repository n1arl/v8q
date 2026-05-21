# Window Capture

Window capture is currently Hyprland-only. V8Q reads windows with:

```bash
hyprctl clients -j
```

List windows:

```bash
v8q windows
v8q windows --json
```

Select a window:

```bash
v8q window select --title Firefox
v8q window select --class firefox
v8q window select --app-id steam
v8q window select --interactive
v8q window show
```

If multiple windows match in non-interactive mode, V8Q prints the candidates and asks for a more specific filter. Interactive mode lists windows and asks for a number.

Record only the active window for one run:

```bash
v8q start --target active-window
```

V8Q passes the selected rectangle to `wl-screenrec --geometry`. If the window moves, run `v8q window select` again, or use `follow = true` to resolve it before start.

Window capture requires Hyprland and `hyprctl clients -j`. `v8q doctor --verbose` checks whether the local `wl-screenrec` supports `--geometry`.

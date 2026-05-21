# Hyprland

Recommended binds:

```conf
bind = SUPER_SHIFT, R, exec, v8q save
bind = SUPER_SHIFT, S, exec, v8q start
bind = SUPER_SHIFT, X, exec, v8q stop
bind = SUPER_SHIFT, D, exec, v8q doctor
```

Run:

```bash
v8q setup hyprland
```

Use `--write` only if you want V8Q to append the block to `~/.config/hypr/hyprland.conf`.

Window capture:

```bash
v8q windows
v8q window select --interactive
v8q window show
```

V8Q uses `hyprctl clients -j` and passes the selected rectangle to `wl-screenrec --geometry`.

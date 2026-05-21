# Config

Config path:

```text
~/.config/v8q/config.toml
```

Useful commands:

```bash
v8q config path
v8q config show
v8q config show --json
v8q config validate
v8q config edit
v8q config migrate
v8q config migrate --write
```

`config migrate` is a dry run by default. `config migrate --write` creates a timestamped backup before writing changes, including legacy `[capture_window]` to `[capture.window]` migration.

Beginner/advanced mode:

```bash
v8q mode beginner
v8q mode advanced
v8q mode show
```

Window capture config:

```toml
[capture.window]
enabled = true
title = "Firefox"
class = "firefox"
address = "0x..."
geometry = "1366,0 1920x1080"
follow = false
```

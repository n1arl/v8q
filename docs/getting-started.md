# Getting Started

V8Q is Linux-only for now. The best-tested path is Arch/Omarchy with Hyprland.

Install locally:

```bash
cargo install --path .
```

If `v8q` is not found, add Cargo's bin directory to your shell:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

First-run flow:

```bash
v8q welcome
v8q setup
v8q doctor
v8q preset apply beginner-safe --write
v8q debug wl-screenrec --test-run 5
```

Basic loop:

```bash
v8q start
v8q save --name first-clip
v8q clips
v8q stop
```

Hyprland binds:

```bash
v8q setup hyprland
```

# Contributing

V8Q is a small Linux replay recorder. Keep changes focused, CLI-first, and easy to debug.

Before opening a PR:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

The default build must not require GTK. GUI work belongs behind the `gui` feature.

Good first areas:

- docs and troubleshooting for Hyprland setups
- backend compatibility notes
- focused CLI UX improvements
- tests around config, window selection, and process handling

# Release Checklist

Use this before publishing a V8Q release or release candidate.

## Validation Commands

```bash
cargo fmt
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
cargo install --path . --force
```

Optional GUI build:

```bash
cargo build --features gui --bin v8q-gui
```

## Manual Smoke Test

Use the conservative preset first:

```bash
v8q doctor
v8q preset apply beginner-safe --write
v8q debug wl-screenrec --test-run 5
v8q start
sleep 10
v8q save --name release-smoke --json
v8q stop
v8q clips --latest
v8q clip info "$(v8q clips --latest)"
```

Expected:

- `doctor` has no FAIL items.
- `debug wl-screenrec --test-run 5` writes a non-empty history file.
- `save` creates a clip in `~/Videos/V8Q`.
- `clip info` shows a readable video stream.
- `v8q status` does not report a stale PID after stop.

If it fails, collect:

```bash
v8q debug report
v8q logs --lines 100
v8q doctor --verbose
```

## Pre-Publish Checklist

- README is short and links to detailed docs.
- `CHANGELOG.md` has the release section updated.
- `Cargo.toml` version is correct.
- `cargo install --path .` installs only the CLI by default.
- GUI build is optional and documented.
- Known `wl-screenrec`/NVIDIA limitations are documented honestly.
- Issue templates ask for `v8q debug report`.
- No local configs, logs, target artifacts, or clips are staged.

## Create Tag

For a release candidate:

```bash
git status --short
git add .
git commit -m "Prepare V8Q v0.4.0 release candidate"
git tag v0.4.0-rc3
git push origin main
git push origin v0.4.0-rc3
```

For a final release:

```bash
git tag v0.4.0
git push origin v0.4.0
```

## Create GitHub Release

1. Open GitHub releases.
2. Draft a new release from the tag.
3. Mark release candidates as pre-release.
4. Paste the release notes from [release-notes-v0.4.0-rc3.md](release-notes-v0.4.0-rc3.md).
5. Include the smoke test command block.
6. Publish.

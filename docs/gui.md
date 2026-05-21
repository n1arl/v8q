# GUI

The GUI is optional and experimental. It uses GTK4/libadwaita, not Electron.

Install runtime/build dependencies on Arch:

```bash
sudo pacman -S gtk4 libadwaita
```

Build:

```bash
cargo build --features gui --bin v8q-gui
```

Run:

```bash
cargo run --features gui --bin v8q-gui
```

The CLI remains the core. The GUI calls the same public Rust functions.

Current status:

- shows recorder status and main config values
- Start, Save Replay, Stop, Refresh, Open Clips Folder, Doctor, and Settings call the shared core
- long-running actions run outside the GTK main thread

TODO before the GUI is considered polished:

- show the latest 5 clips
- add capture target controls for fullscreen vs window
- add select/clear window controls
- add an easier "copy error details" action
- add a clearer log viewer inside the GUI

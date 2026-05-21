use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

#[cfg(feature = "gui")]
use gtk4::{glib, prelude::*};

#[cfg(feature = "gui")]
#[derive(Debug)]
enum UiEvent {
    Snapshot(Result<Snapshot, String>),
    Started(Result<(v8q::StartResult, Snapshot), String>),
    Saved(Result<(v8q::SaveResult, Snapshot), String>),
    Stopped(Result<(v8q::StopResult, Snapshot), String>),
    Doctor(Result<v8q::DoctorReport, String>),
    OpenedFolder(Result<(), String>),
    OpenedConfig(Result<(), String>),
}

#[cfg(feature = "gui")]
#[derive(Debug, Clone)]
struct Snapshot {
    status: v8q::StatusInfo,
    latest_clip: Option<std::path::PathBuf>,
}

#[cfg(feature = "gui")]
struct Ui {
    config: v8q::Config,
    window: libadwaita::ApplicationWindow,
    status_value: gtk4::Label,
    pid_value: gtk4::Label,
    backend_value: gtk4::Label,
    capture_target_value: gtk4::Label,
    replay_value: gtk4::Label,
    fps_value: gtk4::Label,
    encoder_value: gtk4::Label,
    bitrate_value: gtk4::Label,
    buffer_value: gtk4::Label,
    output_value: gtk4::Label,
    latest_clip_value: gtk4::Label,
    recent_errors_value: gtk4::Label,
    message_value: gtk4::Label,
    start_button: gtk4::Button,
    save_button: gtk4::Button,
    stop_button: gtk4::Button,
    copy_error_button: gtk4::Button,
    open_logs_button: gtk4::Button,
    last_error: Rc<RefCell<String>>,
}

#[cfg(feature = "gui")]
fn main() -> anyhow::Result<()> {
    let app = libadwaita::Application::builder()
        .application_id("dev.v8q.V8Q")
        .build();

    app.connect_activate(build_ui);
    app.run();
    Ok(())
}

#[cfg(feature = "gui")]
fn build_ui(app: &libadwaita::Application) {
    let config = match v8q::load_or_create_default_config() {
        Ok(config) => config,
        Err(error) => {
            show_startup_error(app, &format!("Failed to load V8Q config:\n{error:#}"));
            return;
        }
    };

    let (tx, rx) = mpsc::channel::<UiEvent>();

    let title = gtk4::Label::builder()
        .label("V8Q")
        .xalign(0.0)
        .css_classes(["title-1"])
        .build();

    let grid = gtk4::Grid::builder()
        .column_spacing(14)
        .row_spacing(8)
        .hexpand(true)
        .build();

    let status_value = add_row(&grid, 0, "Status");
    let pid_value = add_row(&grid, 1, "PID");
    let backend_value = add_row(&grid, 2, "Backend");
    let capture_target_value = add_row(&grid, 3, "Capture target");
    let replay_value = add_row(&grid, 4, "Replay");
    let fps_value = add_row(&grid, 5, "FPS");
    let encoder_value = add_row(&grid, 6, "Encoder");
    let bitrate_value = add_row(&grid, 7, "Bitrate");
    let buffer_value = add_row(&grid, 8, "Buffer");
    let output_value = add_row(&grid, 9, "Clips");
    let latest_clip_value = add_row(&grid, 10, "Last clip");
    let recent_errors_value = add_row(&grid, 11, "Recent errors");

    let message_value = gtk4::Label::builder()
        .label("Ready.")
        .xalign(0.0)
        .selectable(true)
        .wrap(true)
        .build();

    let start_button = gtk4::Button::with_label("Start");
    let save_button = gtk4::Button::with_label("Save Replay");
    let stop_button = gtk4::Button::with_label("Stop");
    let refresh_button = gtk4::Button::with_label("Refresh");
    let open_folder_button = gtk4::Button::with_label("Open Clips Folder");
    let doctor_button = gtk4::Button::with_label("Doctor");
    let settings_button = gtk4::Button::with_label("Settings");
    let copy_error_button = gtk4::Button::with_label("Copy Error Details");
    let open_logs_button = gtk4::Button::with_label("Open Logs");
    copy_error_button.set_sensitive(false);

    let primary_buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    primary_buttons.append(&start_button);
    primary_buttons.append(&save_button);
    primary_buttons.append(&stop_button);

    let secondary_buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    secondary_buttons.append(&refresh_button);
    secondary_buttons.append(&open_folder_button);
    secondary_buttons.append(&doctor_button);
    secondary_buttons.append(&settings_button);
    secondary_buttons.append(&copy_error_button);
    secondary_buttons.append(&open_logs_button);

    let root = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(16)
        .margin_top(18)
        .margin_bottom(18)
        .margin_start(18)
        .margin_end(18)
        .build();
    root.append(&title);
    root.append(&grid);
    root.append(&message_value);
    root.append(&primary_buttons);
    root.append(&secondary_buttons);

    let window = libadwaita::ApplicationWindow::builder()
        .application(app)
        .title("V8Q")
        .default_width(680)
        .default_height(440)
        .content(&root)
        .build();

    let ui = Rc::new(Ui {
        config,
        window,
        status_value,
        pid_value,
        backend_value,
        capture_target_value,
        replay_value,
        fps_value,
        encoder_value,
        bitrate_value,
        buffer_value,
        output_value,
        latest_clip_value,
        recent_errors_value,
        message_value,
        start_button,
        save_button,
        stop_button,
        copy_error_button,
        open_logs_button,
        last_error: Rc::new(RefCell::new(String::new())),
    });

    let last_status = Rc::new(RefCell::new(None::<v8q::StatusInfo>));

    connect_actions(
        &ui,
        &tx,
        &refresh_button,
        &open_folder_button,
        &doctor_button,
        &settings_button,
        &last_status,
    );

    ui.window.present();
    spawn_snapshot(ui.config.clone(), tx.clone());

    let ui_for_events = Rc::clone(&ui);
    let last_status_for_events = Rc::clone(&last_status);
    glib::timeout_add_local(Duration::from_millis(100), move || {
        while let Ok(event) = rx.try_recv() {
            handle_event(&ui_for_events, &last_status_for_events, event);
        }
        glib::ControlFlow::Continue
    });
}

#[cfg(feature = "gui")]
fn connect_actions(
    ui: &Rc<Ui>,
    tx: &mpsc::Sender<UiEvent>,
    refresh_button: &gtk4::Button,
    open_folder_button: &gtk4::Button,
    doctor_button: &gtk4::Button,
    settings_button: &gtk4::Button,
    last_status: &Rc<RefCell<Option<v8q::StatusInfo>>>,
) {
    let config = ui.config.clone();
    let tx_start = tx.clone();
    ui.start_button.connect_clicked(move |_| {
        let config = config.clone();
        let tx = tx_start.clone();
        std::thread::spawn(move || {
            let result = v8q::start_recorder(&config)
                .and_then(|start| snapshot(&config).map(|snapshot| (start, snapshot)))
                .map_err(format_error);
            let _ = tx.send(UiEvent::Started(result));
        });
    });

    let config = ui.config.clone();
    let tx_save = tx.clone();
    ui.save_button.connect_clicked(move |_| {
        let config = config.clone();
        let tx = tx_save.clone();
        std::thread::spawn(move || {
            let result = v8q::save_replay(&config)
                .map(|save| {
                    v8q::notify_replay_saved(&config, &save.output_file);
                    save
                })
                .and_then(|save| snapshot(&config).map(|snapshot| (save, snapshot)))
                .map_err(format_error);
            let _ = tx.send(UiEvent::Saved(result));
        });
    });

    let config = ui.config.clone();
    let tx_stop = tx.clone();
    ui.stop_button.connect_clicked(move |_| {
        let config = config.clone();
        let tx = tx_stop.clone();
        std::thread::spawn(move || {
            let result = v8q::stop_recorder(&config)
                .and_then(|stop| snapshot(&config).map(|snapshot| (stop, snapshot)))
                .map_err(format_error);
            let _ = tx.send(UiEvent::Stopped(result));
        });
    });

    let config = ui.config.clone();
    let tx_refresh = tx.clone();
    refresh_button.connect_clicked(move |_| {
        spawn_snapshot(config.clone(), tx_refresh.clone());
    });

    let config = ui.config.clone();
    let tx_open = tx.clone();
    open_folder_button.connect_clicked(move |_| {
        let config = config.clone();
        let tx = tx_open.clone();
        std::thread::spawn(move || {
            let result = v8q::open_path(config.paths.output_dir_path()).map_err(format_error);
            let _ = tx.send(UiEvent::OpenedFolder(result));
        });
    });

    let config = ui.config.clone();
    let tx_doctor = tx.clone();
    doctor_button.connect_clicked(move |_| {
        let config = config.clone();
        let tx = tx_doctor.clone();
        std::thread::spawn(move || {
            let result = v8q::run_doctor(&config).map_err(format_error);
            let _ = tx.send(UiEvent::Doctor(result));
        });
    });

    let config = ui.config.clone();
    let ui_settings = Rc::clone(ui);
    let tx_settings = tx.clone();
    let last_status = Rc::clone(last_status);
    settings_button.connect_clicked(move |_| {
        show_settings_dialog(
            &ui_settings,
            &config,
            last_status.borrow().as_ref(),
            tx_settings.clone(),
        );
    });

    let last_error = Rc::clone(&ui.last_error);
    ui.copy_error_button.connect_clicked(move |_| {
        if let Some(display) = gtk4::gdk::Display::default() {
            display.clipboard().set_text(&last_error.borrow());
        }
    });

    let tx_logs = tx.clone();
    ui.open_logs_button.connect_clicked(move |_| {
        let tx = tx_logs.clone();
        std::thread::spawn(move || {
            let result = v8q::open_path(v8q::paths::logs_dir()).map_err(format_error);
            let _ = tx.send(UiEvent::OpenedFolder(result));
        });
    });
}

#[cfg(feature = "gui")]
fn handle_event(ui: &Rc<Ui>, last_status: &Rc<RefCell<Option<v8q::StatusInfo>>>, event: UiEvent) {
    match event {
        UiEvent::Snapshot(Ok(snapshot)) => {
            apply_snapshot(ui, last_status, snapshot);
            set_message(ui, "Status refreshed.");
        }
        UiEvent::Snapshot(Err(error)) => set_error(ui, &error),
        UiEvent::Started(Ok((start, snapshot))) => {
            apply_snapshot(ui, last_status, snapshot);
            set_message(
                ui,
                &format!(
                    "Started recorder PID {} using {}. Log: {}",
                    start.pid,
                    start.backend,
                    start.log_file.display()
                ),
            );
        }
        UiEvent::Started(Err(error)) => set_error(ui, &error),
        UiEvent::Saved(Ok((save, snapshot))) => {
            apply_snapshot(ui, last_status, snapshot);
            set_message(ui, &format!("Saved replay: {}", save.output_file.display()));
        }
        UiEvent::Saved(Err(error)) => set_error(ui, &error),
        UiEvent::Stopped(Ok((stop, snapshot))) => {
            apply_snapshot(ui, last_status, snapshot);
            let message = if stop.was_running {
                format!("Stopped recorder PID {}.", stop.pid.unwrap_or_default())
            } else {
                "Recorder was already stopped.".to_string()
            };
            set_message(ui, &message);
        }
        UiEvent::Stopped(Err(error)) => set_error(ui, &error),
        UiEvent::Doctor(Ok(report)) => show_doctor_dialog(ui, &report),
        UiEvent::Doctor(Err(error)) => set_error(ui, &error),
        UiEvent::OpenedFolder(Ok(())) => set_message(ui, "Opened clips folder."),
        UiEvent::OpenedFolder(Err(error)) => set_error(ui, &error),
        UiEvent::OpenedConfig(Ok(())) => set_message(ui, "Opened config file."),
        UiEvent::OpenedConfig(Err(error)) => set_error(ui, &error),
    }
}

#[cfg(feature = "gui")]
fn apply_snapshot(
    ui: &Rc<Ui>,
    last_status: &Rc<RefCell<Option<v8q::StatusInfo>>>,
    snapshot: Snapshot,
) {
    let status = snapshot.status;
    ui.status_value.set_label(if status.is_running {
        "Recording"
    } else {
        "Stopped"
    });
    ui.pid_value.set_label(
        &status
            .pid
            .map(|pid| pid.to_string())
            .unwrap_or_else(|| "-".to_string()),
    );
    ui.backend_value.set_label(&status.backend);
    ui.capture_target_value.set_label(&status.capture_target);
    ui.replay_value
        .set_label(&format!("{}s", status.replay_duration));
    ui.fps_value.set_label(&status.fps.to_string());
    ui.encoder_value.set_label(&status.encoder);
    ui.bitrate_value.set_label(&status.bitrate);
    ui.buffer_value
        .set_label(&status.buffer_dir.to_string_lossy());
    ui.output_value
        .set_label(&status.output_dir.to_string_lossy());
    ui.latest_clip_value.set_label(
        &snapshot
            .latest_clip
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| "-".to_string()),
    );
    ui.recent_errors_value
        .set_label(&if status.last_error_lines.is_empty() {
            "-".to_string()
        } else {
            status.last_error_lines.join("\n")
        });
    let mut messages = status.warnings.clone();
    if status.is_running && status.history_exists == Some(false) {
        messages.push(
            "Recorder process is running, but no replay file has been produced yet.".to_string(),
        );
    }
    if !messages.is_empty() {
        set_message(ui, &messages.join("\n"));
    }

    ui.start_button.set_sensitive(!status.is_running);
    ui.stop_button.set_sensitive(status.is_running);
    // Keep save tied to an active recorder. It avoids implying stale segment export is reliable from the GUI.
    ui.save_button.set_sensitive(status.is_running);

    *last_status.borrow_mut() = Some(status);
}

#[cfg(feature = "gui")]
fn show_doctor_dialog(ui: &Rc<Ui>, report: &v8q::DoctorReport) {
    let content = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(10)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .build();

    let summary = gtk4::Label::builder()
        .label(format!(
            "Summary: {} OK, {} WARN, {} FAIL",
            report.ok_count, report.warn_count, report.fail_count
        ))
        .xalign(0.0)
        .build();
    content.append(&summary);

    for check in &report.checks {
        let status = match check.status {
            v8q::DoctorCheckStatus::Ok => "OK",
            v8q::DoctorCheckStatus::Warn => "WARN",
            v8q::DoctorCheckStatus::Fail => "FAIL",
        };
        let text = if let Some(hint) = &check.hint {
            format!("[{status}] {}\n{}\n{}", check.name, check.message, hint)
        } else {
            format!("[{status}] {}\n{}", check.name, check.message)
        };
        content.append(
            &gtk4::Label::builder()
                .label(text)
                .xalign(0.0)
                .selectable(true)
                .wrap(true)
                .build(),
        );
    }

    present_child_window(ui, "V8Q Doctor", &content, 720, 620);
}

#[cfg(feature = "gui")]
fn show_settings_dialog(
    ui: &Rc<Ui>,
    config: &v8q::Config,
    status: Option<&v8q::StatusInfo>,
    tx: mpsc::Sender<UiEvent>,
) {
    let backend = config
        .effective_backend()
        .map(|backend| backend.as_str().to_string())
        .unwrap_or_else(|error| format!("invalid ({error})"));
    let config_path = v8q::config_path().ok();

    let text = format!(
        "Config: {}\nBackend: {}\nReplay duration: {}s\nSegment duration: {}s\nFPS: {}\nEncoder: {}\nBitrate: {}\nAudio: {}\nOutput: {}\nBuffer: {}",
        config_path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| "-".to_string()),
        backend,
        config.recording.duration_seconds,
        config.recording.segment_seconds,
        config.recording.fps,
        status
            .map(|status| status.encoder.clone())
            .unwrap_or_else(|| config.recording.encoder.clone()),
        status
            .map(|status| status.bitrate.clone())
            .unwrap_or_else(|| config.recording.video_bitrate.clone()),
        if config.wl_screenrec.audio { "on" } else { "off" },
        config.paths.output_dir_path().display(),
        config.paths.buffer_dir_path().display(),
    );

    let content = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(12)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .build();
    content.append(
        &gtk4::Label::builder()
            .label(text)
            .xalign(0.0)
            .selectable(true)
            .wrap(true)
            .build(),
    );

    let open_button = gtk4::Button::with_label("Open Config File");
    open_button.connect_clicked(move |_| {
        let tx = tx.clone();
        std::thread::spawn(move || {
            let result = v8q::open_config_file().map_err(format_error);
            let _ = tx.send(UiEvent::OpenedConfig(result));
        });
    });
    content.append(&open_button);
    present_child_window(ui, "V8Q Settings", &content, 620, 360);
}

#[cfg(feature = "gui")]
fn present_child_window(
    ui: &Rc<Ui>,
    title: &str,
    content: &impl IsA<gtk4::Widget>,
    width: i32,
    height: i32,
) {
    let window = gtk4::Window::builder()
        .title(title)
        .default_width(width)
        .default_height(height)
        .transient_for(&ui.window)
        .modal(true)
        .child(content)
        .build();
    window.present();
}

#[cfg(feature = "gui")]
fn spawn_snapshot(config: v8q::Config, tx: mpsc::Sender<UiEvent>) {
    std::thread::spawn(move || {
        let result = snapshot(&config).map_err(format_error);
        let _ = tx.send(UiEvent::Snapshot(result));
    });
}

#[cfg(feature = "gui")]
fn snapshot(config: &v8q::Config) -> anyhow::Result<Snapshot> {
    Ok(Snapshot {
        status: v8q::get_status(config)?,
        latest_clip: v8q::latest_clip(config)?,
    })
}

#[cfg(feature = "gui")]
fn add_row(grid: &gtk4::Grid, row: i32, name: &str) -> gtk4::Label {
    let label = gtk4::Label::builder()
        .label(format!("{name}:"))
        .xalign(0.0)
        .build();
    let value = gtk4::Label::builder()
        .label("-")
        .xalign(0.0)
        .selectable(true)
        .hexpand(true)
        .ellipsize(gtk4::pango::EllipsizeMode::Middle)
        .build();
    grid.attach(&label, 0, row, 1, 1);
    grid.attach(&value, 1, row, 1, 1);
    value
}

#[cfg(feature = "gui")]
fn set_message(ui: &Rc<Ui>, message: &str) {
    ui.message_value.set_label(message);
    ui.last_error.borrow_mut().clear();
    ui.copy_error_button.set_sensitive(false);
}

#[cfg(feature = "gui")]
fn set_error(ui: &Rc<Ui>, message: &str) {
    ui.message_value.set_label(&format!("Error: {message}"));
    *ui.last_error.borrow_mut() = message.to_string();
    ui.copy_error_button.set_sensitive(true);
}

#[cfg(feature = "gui")]
fn format_error(error: anyhow::Error) -> String {
    format!("{error:#}")
}

#[cfg(feature = "gui")]
fn show_startup_error(app: &libadwaita::Application, message: &str) {
    let label = gtk4::Label::builder()
        .label(message)
        .wrap(true)
        .selectable(true)
        .margin_top(18)
        .margin_bottom(18)
        .margin_start(18)
        .margin_end(18)
        .build();
    let window = libadwaita::ApplicationWindow::builder()
        .application(app)
        .title("V8Q")
        .default_width(520)
        .default_height(220)
        .content(&label)
        .build();
    window.present();
}

#[cfg(not(feature = "gui"))]
fn main() {
    eprintln!(
        "v8q-gui requires the gui feature. Build with: cargo build --features gui --bin v8q-gui"
    );
    std::process::exit(1);
}

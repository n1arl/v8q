use std::process::Command;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::Config;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WindowInfo {
    pub title: String,
    pub class: String,
    pub app_id: String,
    pub workspace: String,
    pub pid: Option<u32>,
    pub address: String,
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
}

impl WindowInfo {
    pub fn geometry(&self) -> String {
        format!("{},{} {}x{}", self.x, self.y, self.width, self.height)
    }
}

pub fn list_hyprland_windows() -> anyhow::Result<Vec<WindowInfo>> {
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none()
        && std::env::var("XDG_CURRENT_DESKTOP")
            .map(|desktop| !desktop.to_lowercase().contains("hyprland"))
            .unwrap_or(true)
    {
        anyhow::bail!("Window selection is currently supported on Hyprland through hyprctl.");
    }
    let output = Command::new("hyprctl")
        .args(["clients", "-j"])
        .output()
        .context("failed to run hyprctl clients -j")?;
    if !output.status.success() {
        anyhow::bail!(
            "hyprctl clients -j failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    parse_hyprctl_clients_json(&String::from_utf8_lossy(&output.stdout))
}

pub fn parse_hyprctl_clients_json(text: &str) -> anyhow::Result<Vec<WindowInfo>> {
    let clients: Vec<serde_json::Value> =
        serde_json::from_str(text).context("failed to parse hyprctl clients -j JSON")?;
    Ok(clients
        .iter()
        .filter_map(window_from_value)
        .collect::<Vec<_>>())
}

fn window_from_value(value: &serde_json::Value) -> Option<WindowInfo> {
    let at = value.get("at")?.as_array()?;
    let size = value.get("size")?.as_array()?;
    let x = json_i64(at.first()?)?;
    let y = json_i64(at.get(1)?)?;
    let width = json_i64(size.first()?)?;
    let height = json_i64(size.get(1)?)?;
    Some(WindowInfo {
        title: string_field(value, "title"),
        class: string_field(value, "class"),
        app_id: string_field(value, "initialClass"),
        workspace: value
            .get("workspace")
            .and_then(|workspace| workspace.get("name"))
            .and_then(|name| name.as_str())
            .unwrap_or("")
            .to_string(),
        pid: value
            .get("pid")
            .and_then(|pid| pid.as_u64())
            .and_then(|pid| u32::try_from(pid).ok()),
        address: string_field(value, "address"),
        x,
        y,
        width,
        height,
    })
}

fn string_field(value: &serde_json::Value, field: &str) -> String {
    value
        .get(field)
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string()
}

fn json_i64(value: &serde_json::Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
        .or_else(|| value.as_f64().map(|value| value.round() as i64))
}

pub fn select_window(
    windows: &[WindowInfo],
    title: Option<&str>,
    class: Option<&str>,
    app_id: Option<&str>,
) -> anyhow::Result<WindowInfo> {
    let mut matches = windows
        .iter()
        .filter(|window| {
            title
                .map(|needle| contains_ci(&window.title, needle))
                .unwrap_or(true)
                && class
                    .map(|needle| contains_ci(&window.class, needle))
                    .unwrap_or(true)
                && app_id
                    .map(|needle| {
                        contains_ci(&window.app_id, needle) || contains_ci(&window.class, needle)
                    })
                    .unwrap_or(true)
        })
        .cloned()
        .collect::<Vec<_>>();

    if matches.is_empty() {
        anyhow::bail!("no window matched the requested filter");
    }
    if matches.len() > 1 {
        matches.truncate(10);
        let candidates = matches
            .iter()
            .map(|window| {
                format!(
                    "- title='{}' class='{}' address={} geometry={}",
                    window.title,
                    window.class,
                    window.address,
                    window.geometry()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        anyhow::bail!("multiple windows matched. Use --title or --app-id with a more specific value:\n{candidates}");
    }
    Ok(matches.remove(0))
}

pub fn selected_window_geometry(config: &Config) -> anyhow::Result<Option<String>> {
    let window = config.effective_capture_window();
    if !window.enabled {
        return Ok(None);
    }
    if window.follow {
        let windows = list_hyprland_windows()?;
        let selected = if !window.address.trim().is_empty() {
            windows
                .iter()
                .find(|candidate| candidate.address == window.address)
                .cloned()
        } else {
            Some(select_window(
                &windows,
                (!window.title.trim().is_empty()).then_some(window.title.as_str()),
                (!window.class.trim().is_empty()).then_some(window.class.as_str()),
                None,
            )?)
        };
        return selected
            .map(|window| Some(window.geometry()))
            .ok_or_else(|| {
                anyhow::anyhow!("configured window was not found; run `v8q window select` again")
            });
    }
    if window.geometry.trim().is_empty() {
        anyhow::bail!(
            "capture.window.enabled is true, but geometry is empty; run `v8q window select`"
        );
    }
    Ok(Some(window.geometry.clone()))
}

fn contains_ci(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

pub fn apply_selected_window(config: &mut Config, window: &WindowInfo) {
    let capture = config
        .capture
        .get_or_insert_with(crate::config::CaptureConfig::default);
    capture.window = Some(crate::config::CaptureWindowConfig {
        enabled: true,
        title: window.title.clone(),
        class: window.class.clone(),
        address: window.address.clone(),
        geometry: window.geometry(),
        follow: false,
    });
    config.capture_window = None;
    config.wl_screenrec.capture_mode = "geometry".to_string();
    config.wl_screenrec.geometry = window.geometry();
}

#[cfg(test)]
mod tests {
    use super::{parse_hyprctl_clients_json, select_window};

    const CLIENTS: &str = r#"[
      {
        "address": "0xabc",
        "at": [1366, 0],
        "size": [1920, 1080],
        "workspace": {"name": "1"},
        "class": "firefox",
        "initialClass": "firefox",
        "title": "Mozilla Firefox",
        "pid": 1234
      },
      {
        "address": "0xdef",
        "at": [0, 0],
        "size": [1366, 768],
        "workspace": {"name": "2"},
        "class": "steam",
        "initialClass": "steam",
        "title": "Steam",
        "pid": 2345
      }
    ]"#;

    #[test]
    fn parses_hyprctl_clients() {
        let windows = parse_hyprctl_clients_json(CLIENTS).unwrap();
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].geometry(), "1366,0 1920x1080");
    }

    #[test]
    fn selects_window_by_title() {
        let windows = parse_hyprctl_clients_json(CLIENTS).unwrap();
        let selected = select_window(&windows, Some("Firefox"), None, None).unwrap();
        assert_eq!(selected.class, "firefox");
    }

    #[test]
    fn selects_window_by_class() {
        let windows = parse_hyprctl_clients_json(CLIENTS).unwrap();
        let selected = select_window(&windows, None, Some("steam"), None).unwrap();
        assert_eq!(selected.title, "Steam");
    }

    #[test]
    fn select_window_errors_when_none_match() {
        let windows = parse_hyprctl_clients_json(CLIENTS).unwrap();
        assert!(select_window(&windows, Some("missing"), None, None).is_err());
    }

    #[test]
    fn select_window_errors_when_multiple_match() {
        let windows = parse_hyprctl_clients_json(CLIENTS).unwrap();
        assert!(select_window(&windows, None, None, None).is_err());
    }
}

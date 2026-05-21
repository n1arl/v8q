use std::path::PathBuf;
use std::process::Command;

use anyhow::Context;

pub fn service_path() -> anyhow::Result<PathBuf> {
    let dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("could not locate config dir"))?
        .join("systemd/user");
    Ok(dir.join("v8q.service"))
}

pub fn service_file() -> &'static str {
    "[Unit]\nDescription=V8Q Replay Recorder\nAfter=graphical-session.target\n\n[Service]\nType=simple\nExecStart=%h/.cargo/bin/v8q start --foreground\nExecStop=%h/.cargo/bin/v8q stop\nRestart=on-failure\nRestartSec=3\n\n[Install]\nWantedBy=default.target\n"
}

pub fn install() -> anyhow::Result<PathBuf> {
    let path = service_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    std::fs::write(&path, service_file())
        .with_context(|| format!("failed to write {}", path.display()))?;
    let _ = systemctl(&["daemon-reload"]);
    Ok(path)
}

pub fn uninstall() -> anyhow::Result<PathBuf> {
    let path = service_path()?;
    let _ = systemctl(&["--now", "disable", "v8q.service"]);
    let _ = std::fs::remove_file(&path);
    let _ = systemctl(&["daemon-reload"]);
    Ok(path)
}

pub fn systemctl(args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .output()
        .with_context(|| "failed to run systemctl --user")?;
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    if !output.status.success() {
        anyhow::bail!(
            "systemctl --user {} failed: {}",
            args.join(" "),
            text.trim()
        );
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    #[test]
    fn service_file_contains_foreground() {
        assert!(super::service_file().contains("v8q start --foreground"));
    }
}

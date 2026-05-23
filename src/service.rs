use crate::cli::{ServiceArgs, ServiceRemoveArgs};
use anyhow::{bail, Context, Result};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(target_os = "linux")]
const SERVICE_NAME: &str = "rtk-sync";
#[cfg(target_os = "macos")]
const MACOS_LABEL: &str = "com.vuong.rtk-sync";

pub fn install(args: ServiceArgs) -> Result<()> {
    let binary = resolve_binary(args.binary)?;
    ensure_absolute_binary(&binary)?;

    match env::consts::OS {
        "macos" => install_launchd(&binary, args.config.as_deref()),
        "linux" => install_systemd(&binary, args.config.as_deref()),
        os => bail!("install-service is not supported on {os}"),
    }
}

pub fn uninstall(_args: ServiceRemoveArgs) -> Result<()> {
    match env::consts::OS {
        "macos" => uninstall_launchd(),
        "linux" => uninstall_systemd(),
        os => bail!("uninstall-service is not supported on {os}"),
    }
}

fn resolve_binary(cli_binary: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(binary) = cli_binary {
        return Ok(binary);
    }
    env::current_exe().context("failed to resolve current executable path")
}

fn ensure_absolute_binary(binary: &Path) -> Result<()> {
    if !binary.is_absolute() {
        bail!("--binary must be an absolute path: {}", binary.display());
    }
    if !binary.exists() {
        bail!("binary does not exist: {}", binary.display());
    }
    Ok(())
}

fn service_args(binary: &Path, config: Option<&Path>) -> Vec<String> {
    let mut args = vec![binary.display().to_string(), "run-service".to_string()];
    if let Some(config) = config {
        args.push("--config".to_string());
        args.push(config.display().to_string());
    }
    args
}

#[cfg(target_os = "macos")]
fn install_launchd(binary: &Path, config: Option<&Path>) -> Result<()> {
    let plist_path = launchd_plist_path()?;
    if let Some(parent) = plist_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create launchd directory: {}", parent.display()))?;
    }
    fs::write(&plist_path, launchd_plist(binary, config))
        .with_context(|| format!("failed to write launchd plist: {}", plist_path.display()))?;

    run_command(
        "launchctl",
        &["unload", plist_path_string(&plist_path).as_str()],
        false,
    )?;
    run_command(
        "launchctl",
        &["load", plist_path_string(&plist_path).as_str()],
        true,
    )?;
    run_command("launchctl", &["start", MACOS_LABEL], true)?;

    println!("Installed launchd service: {}", plist_path.display());
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn install_launchd(_binary: &Path, _config: Option<&Path>) -> Result<()> {
    bail!("launchd service installation is only supported on macOS")
}

#[cfg(target_os = "macos")]
fn uninstall_launchd() -> Result<()> {
    let plist_path = launchd_plist_path()?;
    run_command("launchctl", &["stop", MACOS_LABEL], false)?;
    run_command(
        "launchctl",
        &["unload", plist_path_string(&plist_path).as_str()],
        false,
    )?;
    if plist_path.exists() {
        fs::remove_file(&plist_path)
            .with_context(|| format!("failed to remove launchd plist: {}", plist_path.display()))?;
    }
    println!("Uninstalled launchd service: {MACOS_LABEL}");
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn uninstall_launchd() -> Result<()> {
    bail!("launchd service removal is only supported on macOS")
}

#[cfg(target_os = "macos")]
fn launchd_plist_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("failed to resolve home directory")?;
    Ok(home
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{MACOS_LABEL}.plist")))
}

#[cfg(target_os = "macos")]
fn launchd_plist(binary: &Path, config: Option<&Path>) -> String {
    let args = service_args(binary, config)
        .into_iter()
        .map(|arg| format!("      <string>{}</string>", escape_xml(&arg)))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
  <dict>
    <key>Label</key>
    <string>{MACOS_LABEL}</string>
    <key>ProgramArguments</key>
    <array>
{args}
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/tmp/rtk-sync.out.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/rtk-sync.err.log</string>
  </dict>
</plist>
"#
    )
}

#[cfg(target_os = "linux")]
fn install_systemd(binary: &Path, config: Option<&Path>) -> Result<()> {
    let service_path = systemd_service_path()?;
    if let Some(parent) = service_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create systemd user directory: {}",
                parent.display()
            )
        })?;
    }
    fs::write(&service_path, systemd_service(binary, config)).with_context(|| {
        format!(
            "failed to write systemd service: {}",
            service_path.display()
        )
    })?;

    run_command("systemctl", &["--user", "daemon-reload"], true)?;
    run_command(
        "systemctl",
        &["--user", "enable", "--now", SERVICE_NAME],
        true,
    )?;

    println!("Installed systemd user service: {}", service_path.display());
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn install_systemd(_binary: &Path, _config: Option<&Path>) -> Result<()> {
    bail!("systemd service installation is only supported on Linux")
}

#[cfg(target_os = "linux")]
fn uninstall_systemd() -> Result<()> {
    let service_path = systemd_service_path()?;
    run_command(
        "systemctl",
        &["--user", "disable", "--now", SERVICE_NAME],
        false,
    )?;
    if service_path.exists() {
        fs::remove_file(&service_path).with_context(|| {
            format!(
                "failed to remove systemd service: {}",
                service_path.display()
            )
        })?;
    }
    run_command("systemctl", &["--user", "daemon-reload"], true)?;
    println!("Uninstalled systemd user service: {SERVICE_NAME}");
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn uninstall_systemd() -> Result<()> {
    bail!("systemd service removal is only supported on Linux")
}

#[cfg(target_os = "linux")]
fn systemd_service_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir().context("failed to resolve config directory")?;
    Ok(config_dir
        .join("systemd")
        .join("user")
        .join(format!("{SERVICE_NAME}.service")))
}

#[cfg(target_os = "linux")]
fn systemd_service(binary: &Path, config: Option<&Path>) -> String {
    let exec_start = service_args(binary, config)
        .into_iter()
        .map(escape_systemd_arg)
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        r#"[Unit]
Description=RTK Sync
After=network-online.target

[Service]
Type=simple
ExecStart={exec_start}
Restart=always
RestartSec=10

[Install]
WantedBy=default.target
"#
    )
}

fn run_command(program: &str, args: &[&str], require_success: bool) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("failed to run {program}"))?;
    if require_success && !status.success() {
        bail!("{program} failed with status {status}");
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn plist_path_string(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(target_os = "macos")]
fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(target_os = "linux")]
fn escape_systemd_arg(value: String) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '-' | '_' | ':' | '='))
    {
        value
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

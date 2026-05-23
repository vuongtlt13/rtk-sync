use crate::cli::UpdateArgs;
use anyhow::{bail, Context, Result};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

const BIN_NAME: &str = "rtk-sync";

pub fn install_latest(args: UpdateArgs) -> Result<()> {
    validate_repo(&args.repo)?;
    let target = release_target()?;
    let archive = format!("{BIN_NAME}-{target}.tar.gz");
    let url = format!(
        "https://github.com/{}/releases/latest/download/{archive}",
        args.repo
    );
    let tmp_dir = create_temp_dir()?;
    let result = install_latest_inner(&args, &url, &archive, &tmp_dir);
    let cleanup_result = fs::remove_dir_all(&tmp_dir)
        .with_context(|| format!("failed to remove temporary directory:{}", tmp_dir.display()));
    result.and(cleanup_result)
}

fn install_latest_inner(args: &UpdateArgs, url: &str, archive: &str, tmp_dir: &Path) -> Result<()> {
    let archive_path = tmp_dir.join(archive);
    let binary_path = tmp_dir.join(BIN_NAME);
    let install_path = args.install_dir.join(BIN_NAME);
    let new_install_path = args.install_dir.join(format!("{BIN_NAME}.new"));
    let plist_path = launchd_plist_path(&args.service_label);

    let service_was_running = stop_service_if_running(&args.service_label, &plist_path)?;

    println!("Downloading {url}");
    run_required(
        "curl",
        &["-fsSL", url, "-o", &archive_path.display().to_string()],
    )?;
    run_required(
        "tar",
        &[
            "-xzf",
            &archive_path.display().to_string(),
            "-C",
            &tmp_dir.display().to_string(),
        ],
    )?;

    if !binary_path.exists() {
        bail!("release archive did not contain {BIN_NAME}");
    }

    fs::create_dir_all(&args.install_dir).with_context(|| {
        format!(
            "failed to create install directory:{}",
            args.install_dir.display()
        )
    })?;

    println!("Installing{}", install_path.display());
    run_required(
        "install",
        &[
            "-m",
            "0755",
            &binary_path.display().to_string(),
            &new_install_path.display().to_string(),
        ],
    )?;
    fs::rename(&new_install_path, &install_path).with_context(|| {
        format!(
            "failed to move{} to{}",
            new_install_path.display(),
            install_path.display()
        )
    })?;

    remove_xattr(&install_path, "com.apple.provenance");
    remove_xattr(&install_path, "com.apple.quarantine");

    run_required(&install_path.display().to_string(), &["--version"])?;

    if service_was_running && args.restart_service {
        start_service(&plist_path)?;
    }

    Ok(())
}

fn validate_repo(repo: &str) -> Result<()> {
    let valid = repo.split('/').count() == 2
        && repo
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/'));
    if !valid {
        bail!("invalid GitHub repo: {repo}");
    }
    Ok(())
}

fn release_target() -> Result<&'static str> {
    match (env::consts::OS, env::consts::ARCH) {
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-gnu"),
        (os, arch) => bail!("unsupported platform: {os}/{arch}"),
    }
}

fn create_temp_dir() -> Result<PathBuf> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before UNIX epoch")?
        .as_nanos();
    let path = env::temp_dir().join(format!(
        "rtk-sync-update-{}-{timestamp}",
        std::process::id()
    ));
    fs::create_dir(&path)
        .with_context(|| format!("failed to create temporary directory:{}", path.display()))?;
    Ok(path)
}

fn launchd_plist_path(service_label: &str) -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{service_label}.plist"))
}

fn stop_service_if_running(service_label: &str, plist_path: &Path) -> Result<bool> {
    if env::consts::OS != "macos" || !plist_path.exists() {
        return Ok(false);
    }

    let domain_label = format!("gui/{}/{service_label}", user_id()?);
    if !command_success("launchctl", &["print", &domain_label])? {
        return Ok(false);
    }

    println!("Stopping {service_label}");
    let domain = format!("gui/{}", user_id()?);
    run_optional(
        "launchctl",
        &["bootout", &domain, &plist_path.display().to_string()],
    )?;
    Ok(true)
}

fn start_service(plist_path: &Path) -> Result<()> {
    println!("Starting service");
    let domain = format!("gui/{}", user_id()?);
    run_required(
        "launchctl",
        &["bootstrap", &domain, &plist_path.display().to_string()],
    )
}

fn user_id() -> Result<String> {
    let output = Command::new("id")
        .arg("-u")
        .output()
        .context("failed to run id -u")?;
    if !output.status.success() {
        bail!("id -u failed with status{}", output.status);
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn remove_xattr(path: &Path, attr: &str) {
    let _ = Command::new("xattr")
        .args(["-d", attr, &path.display().to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn command_success(program: &str, args: &[&str]) -> Result<bool> {
    let status = Command::new(program)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .with_context(|| format!("failed to run {program}"))?;
    Ok(status.success())
}

fn run_optional(program: &str, args: &[&str]) -> Result<()> {
    let _ = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("failed to run {program}"))?;
    Ok(())
}

fn run_required(program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("failed to run {program}"))?;
    if !status.success() {
        bail!("{program} failed with status {status}");
    }
    Ok(())
}

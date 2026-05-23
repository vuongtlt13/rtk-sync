use crate::cli::{ConfigArgs, InspectArgs, MachineIdArgs, ResetArgs, SyncArgs};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const RTK_DATA_DIR: &str = "rtk";
const RTK_HISTORY_DB: &str = "history.db";
const RTK_SYNC_DATA_DIR: &str = "rtk-sync";
const RTK_SYNC_STATE: &str = "state.json";
const RTK_SYNC_CONFIG: &str = "config.toml";
const DEFAULT_TOKEN_ENV: &str = "RTK_SYNC_TOKEN";
const DEFAULT_BATCH_SIZE: usize = 100;
const DEFAULT_INTERVAL_SECONDS: u64 = 60;

#[derive(Debug, Clone)]
pub struct InspectConfig {
    pub db_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct MachineConfig {
    pub state_path: PathBuf,
    pub machine_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SyncConfig {
    pub db_path: PathBuf,
    pub state_path: PathBuf,
    pub endpoint: String,
    pub token_env: String,
    pub token: String,
    pub machine_id: Option<String>,
    pub batch_size: usize,
    pub interval: u64,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct FileConfig {
    pub db: Option<PathBuf>,
    pub state: Option<PathBuf>,
    pub endpoint: Option<String>,
    pub token_env: Option<String>,
    pub token: Option<String>,
    pub machine_id: Option<String>,
    pub batch_size: Option<usize>,
    pub interval: Option<u64>,
    pub allow_insecure_http: Option<bool>,
}

pub fn inspect_config(args: InspectArgs) -> Result<InspectConfig> {
    let file = load_config(args.config.as_deref())?;
    Ok(InspectConfig {
        db_path: resolve_db_path(args.db, &file),
    })
}

pub fn machine_config(args: MachineIdArgs) -> Result<MachineConfig> {
    let file = load_config(args.config.as_deref())?;
    Ok(MachineConfig {
        state_path: resolve_state_path(args.state, &file),
        machine_id: first_string(
            args.machine_id,
            env::var("RTK_SYNC_MACHINE_ID").ok(),
            file.machine_id,
        ),
    })
}

pub fn reset_config(args: ResetArgs) -> Result<PathBuf> {
    let file = load_config(args.config.as_deref())?;
    Ok(resolve_state_path(args.state, &file))
}

pub fn write_config(args: ConfigArgs) -> Result<PathBuf> {
    let path = args.config.unwrap_or_else(default_config_path);
    let mut file = if path.exists() {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        toml::from_str::<FileConfig>(&content)
            .with_context(|| format!("failed to parse config file: {}", path.display()))?
    } else {
        FileConfig::default()
    };

    if let Some(db) = args.db {
        file.db = Some(db);
    }
    if let Some(state) = args.state {
        file.state = Some(state);
    }
    if let Some(endpoint) = args.endpoint {
        file.endpoint = Some(endpoint);
    }
    if let Some(token) = args.token {
        file.token = Some(token);
    }
    if let Some(token_env) = args.token_env {
        file.token_env = Some(token_env);
    }
    if let Some(machine_id) = args.machine_id {
        file.machine_id = Some(machine_id);
    }
    if let Some(batch_size) = args.batch_size {
        if batch_size == 0 {
            bail!("batch size must be greater than zero");
        }
        file.batch_size = Some(batch_size);
    }
    if let Some(interval) = args.interval {
        if interval == 0 {
            bail!("interval must be greater than zero");
        }
        file.interval = Some(interval);
    }
    if let Some(allow_insecure_http) = args.allow_insecure_http {
        file.allow_insecure_http = Some(allow_insecure_http);
    }
    if let Some(endpoint) = &file.endpoint {
        validate_endpoint(endpoint, file.allow_insecure_http.unwrap_or(false))?;
    }

    save_config(&path, &file)?;
    Ok(path)
}

pub fn sync_config(args: SyncArgs) -> Result<SyncConfig> {
    let file = load_config(args.config.as_deref())?;
    let endpoint = first_string(
        args.endpoint,
        env::var("RTK_SYNC_ENDPOINT").ok(),
        file.endpoint.clone(),
    );
    let token_env = first_string(args.token_env, None, file.token_env.clone())
        .unwrap_or_else(|| DEFAULT_TOKEN_ENV.to_string());
    let machine_id = first_string(
        args.machine_id,
        env::var("RTK_SYNC_MACHINE_ID").ok(),
        file.machine_id.clone(),
    );
    let batch_size = args
        .batch_size
        .or_else(|| env::var("RTK_SYNC_BATCH_SIZE").ok()?.parse().ok())
        .or(file.batch_size)
        .unwrap_or(DEFAULT_BATCH_SIZE);
    let interval = file.interval.unwrap_or(DEFAULT_INTERVAL_SECONDS);
    let allow_insecure_http = args.allow_insecure_http || file.allow_insecure_http.unwrap_or(false);

    if batch_size == 0 {
        bail!("batch size must be greater than zero");
    }
    if interval == 0 {
        bail!("interval must be greater than zero");
    }
    let (endpoint, token) = if args.dry_run {
        (endpoint.unwrap_or_default(), String::new())
    } else {
        let endpoint = endpoint.context(
            "endpoint is required; set --endpoint, RTK_SYNC_ENDPOINT, or endpoint in config.toml",
        )?;
        validate_endpoint(&endpoint, allow_insecure_http)?;
        let token = env::var(&token_env).ok().or(file.token.clone()).with_context(|| {
            format!("token is required; set env var {token_env}, token in config.toml, or run rtk-sync config --token <token>")
        })?;
        if token.trim().is_empty() {
            bail!("token is empty");
        }
        (endpoint, token)
    };

    Ok(SyncConfig {
        db_path: resolve_db_path(args.db, &file),
        state_path: resolve_state_path(args.state, &file),
        endpoint,
        token_env,
        token,
        machine_id,
        batch_size,
        interval,
        dry_run: args.dry_run,
    })
}

fn first_string(
    cli_value: Option<String>,
    env_value: Option<String>,
    config_value: Option<String>,
) -> Option<String> {
    cli_value.or(env_value).or(config_value)
}

fn load_config(cli_path: Option<&Path>) -> Result<FileConfig> {
    let path = cli_path
        .map(PathBuf::from)
        .unwrap_or_else(default_config_path);
    if !path.exists() {
        write_default_config(&path)?;
        return Ok(FileConfig::default());
    }

    let content = fs::read_to_string(&path)
        .with_context(|| format!("failed to read config file: {}", path.display()))?;
    toml::from_str(&content)
        .with_context(|| format!("failed to parse config file: {}", path.display()))
}

fn write_default_config(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config directory: {}", parent.display()))?;
    }
    fs::write(path, default_config_content())
        .with_context(|| format!("failed to write default config file: {}", path.display()))
}

fn save_config(path: &Path, config: &FileConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config directory: {}", parent.display()))?;
    }
    let content = toml::to_string_pretty(config).context("failed to serialize config")?;
    fs::write(path, content)
        .with_context(|| format!("failed to write config file: {}", path.display()))
}

fn default_config_content() -> &'static str {
    r#"# rtk-sync config
# This file is created automatically with OS-specific defaults left implicit.
# Environment variables override these values; CLI flags override both.

# db = "/path/to/rtk/history.db"
# state = "/path/to/rtk-sync/state.json"
endpoint = "https://example.com/api/rtk/events"
token_env = "RTK_SYNC_TOKEN"
# token = "replace-me"
# machine_id = "macbook-123124"
batch_size = 100
interval = 60
allow_insecure_http = false
"#
}

fn validate_endpoint(endpoint: &str, allow_insecure_http: bool) -> Result<()> {
    if endpoint.starts_with("https://") || (allow_insecure_http && endpoint.starts_with("http://"))
    {
        return Ok(());
    }

    bail!("endpoint must use https unless --allow-insecure-http is set")
}

fn resolve_db_path(cli_path: Option<PathBuf>, file: &FileConfig) -> PathBuf {
    if let Some(path) = cli_path {
        return path;
    }
    if let Ok(path) = env::var("RTK_SYNC_DB") {
        return PathBuf::from(path);
    }
    if let Ok(path) = env::var("RTK_DB_PATH") {
        return PathBuf::from(path);
    }
    if let Some(path) = &file.db {
        return path.clone();
    }

    let data_dir = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    data_dir.join(RTK_DATA_DIR).join(RTK_HISTORY_DB)
}

fn resolve_state_path(cli_path: Option<PathBuf>, file: &FileConfig) -> PathBuf {
    if let Some(path) = cli_path {
        return path;
    }
    if let Ok(path) = env::var("RTK_SYNC_STATE") {
        return PathBuf::from(path);
    }
    if let Some(path) = &file.state {
        return path.clone();
    }

    let data_dir = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    data_dir.join(RTK_SYNC_DATA_DIR).join(RTK_SYNC_STATE)
}

fn default_config_path() -> PathBuf {
    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    config_dir.join(RTK_SYNC_DATA_DIR).join(RTK_SYNC_CONFIG)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_insecure_http_by_default() {
        assert!(validate_endpoint("http://127.0.0.1:8080", false).is_err());
    }

    #[test]
    fn allows_insecure_http_when_flagged() {
        assert!(validate_endpoint("http://127.0.0.1:8080", true).is_ok());
    }

    #[test]
    fn allows_https() {
        assert!(validate_endpoint("https://example.com/api", false).is_ok());
    }

    #[test]
    fn config_db_overrides_default() {
        let file = FileConfig {
            db: Some(PathBuf::from("/tmp/history.db")),
            ..FileConfig::default()
        };
        assert_eq!(
            resolve_db_path(None, &file),
            PathBuf::from("/tmp/history.db")
        );
    }
}

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "rtk-sync", version, about = "Sync RTK tracking events")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Inspect(InspectArgs),
    MachineId(MachineIdArgs),
    Config(ConfigArgs),
    Reset(ResetArgs),
    Once(SyncArgs),
    #[command(hide = true)]
    RunService(ServiceRunArgs),
    InstallService(ServiceArgs),
    UninstallService(ServiceRemoveArgs),
}

#[derive(Debug, Parser)]
pub struct InspectArgs {
    #[arg(long, env = "RTK_SYNC_CONFIG")]
    pub config: Option<PathBuf>,

    #[arg(long, env = "RTK_SYNC_DB")]
    pub db: Option<PathBuf>,
}

#[derive(Debug, Parser)]
pub struct MachineIdArgs {
    #[arg(long, env = "RTK_SYNC_CONFIG")]
    pub config: Option<PathBuf>,

    #[arg(long, env = "RTK_SYNC_STATE")]
    pub state: Option<PathBuf>,

    #[arg(long, env = "RTK_SYNC_MACHINE_ID")]
    pub machine_id: Option<String>,
}

#[derive(Debug, Parser)]
pub struct ConfigArgs {
    #[arg(long, env = "RTK_SYNC_CONFIG")]
    pub config: Option<PathBuf>,

    #[arg(long)]
    pub endpoint: Option<String>,

    #[arg(long)]
    pub token: Option<String>,

    #[arg(long)]
    pub token_env: Option<String>,

    #[arg(long)]
    pub machine_id: Option<String>,

    #[arg(long)]
    pub batch_size: Option<usize>,

    #[arg(long)]
    pub interval: Option<u64>,

    #[arg(long)]
    pub allow_insecure_http: Option<bool>,

    #[arg(long)]
    pub db: Option<PathBuf>,

    #[arg(long)]
    pub state: Option<PathBuf>,
}

#[derive(Debug, Parser)]
pub struct ResetArgs {
    #[arg(long, env = "RTK_SYNC_CONFIG")]
    pub config: Option<PathBuf>,

    #[arg(long, env = "RTK_SYNC_STATE")]
    pub state: Option<PathBuf>,
}

#[derive(Debug, Clone, Parser)]
pub struct SyncArgs {
    #[arg(long, env = "RTK_SYNC_CONFIG")]
    pub config: Option<PathBuf>,

    #[arg(long, env = "RTK_SYNC_DB")]
    pub db: Option<PathBuf>,

    #[arg(long, env = "RTK_SYNC_STATE")]
    pub state: Option<PathBuf>,

    #[arg(long, env = "RTK_SYNC_ENDPOINT")]
    pub endpoint: Option<String>,

    #[arg(long)]
    pub token_env: Option<String>,

    #[arg(long, env = "RTK_SYNC_MACHINE_ID")]
    pub machine_id: Option<String>,

    #[arg(long, env = "RTK_SYNC_BATCH_SIZE")]
    pub batch_size: Option<usize>,

    #[arg(long, default_value_t = false)]
    pub allow_insecure_http: bool,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Parser)]
pub struct ServiceRunArgs {
    #[arg(long, env = "RTK_SYNC_CONFIG")]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Parser)]
pub struct ServiceArgs {
    #[arg(long)]
    pub binary: Option<PathBuf>,

    #[arg(long, env = "RTK_SYNC_CONFIG")]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Parser)]
pub struct ServiceRemoveArgs {
    #[arg(long, env = "RTK_SYNC_CONFIG")]
    pub config: Option<PathBuf>,
}

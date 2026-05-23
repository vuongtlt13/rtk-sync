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
    /// Inspect the RTK SQLite database schema and diagnostics.
    Inspect(InspectArgs),
    /// Print or initialize the stable local machine ID.
    MachineId(MachineIdArgs),
    /// Update rtk-sync config.toml values.
    Config(ConfigArgs),
    /// Remove the rtk-sync state file and reset the checkpoint.
    Reset(ResetArgs),
    /// Sync one batch of events and exit.
    Once(SyncArgs),
    /// Show config, state, checkpoint, and pending event count.
    Status(StatusArgs),
    /// Download and install the latest rtk-sync release.
    Update(UpdateArgs),
    #[command(hide = true)]
    RunService(ServiceRunArgs),
    /// Install the background sync service.
    InstallService(ServiceArgs),
    /// Uninstall the background sync service.
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

#[derive(Debug, Parser)]
pub struct StatusArgs {
    #[arg(long, env = "RTK_SYNC_CONFIG")]
    pub config: Option<PathBuf>,

    #[arg(long, env = "RTK_SYNC_DB")]
    pub db: Option<PathBuf>,

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
pub struct UpdateArgs {
    #[arg(long, env = "RTK_SYNC_REPO", default_value = "vuongtlt13/rtk-sync")]
    pub repo: String,

    #[arg(
        long,
        env = "RTK_SYNC_INSTALL_DIR",
        default_value = "/opt/homebrew/bin"
    )]
    pub install_dir: PathBuf,

    #[arg(
        long,
        env = "RTK_SYNC_SERVICE_LABEL",
        default_value = "com.vuong.rtk-sync"
    )]
    pub service_label: String,

    #[arg(long, default_value_t = true)]
    pub restart_service: bool,
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

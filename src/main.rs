use anyhow::{Context, Result};
use clap::Parser;
use rtk_sync::cli::{Cli, Command};
use rtk_sync::{config, machine, rtkdb, service, state, syncer, update};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Inspect(args) => {
            let config = config::inspect_config(args)?;
            rtkdb::inspect(&config.db_path)?;
        }
        Command::MachineId(args) => {
            let config = config::machine_config(args)?;
            let mut state = state::State::load(&config.state_path)?;
            let machine_id = machine::resolve_machine_id(config.machine_id.as_deref(), &mut state)?;
            state.save(&config.state_path)?;
            println!("{machine_id}");
        }
        Command::Config(args) => {
            let path = config::write_config(args)?;
            println!("Updated config: {}", path.display());
        }
        Command::Reset(args) => {
            let state_path = config::reset_config(args)?;
            if state_path.exists() {
                std::fs::remove_file(&state_path).with_context(|| {
                    format!("failed to remove state file: {}", state_path.display())
                })?;
                println!("rtk-sync: reset state: {}", state_path.display());
            } else {
                println!("rtk-sync: state already missing: {}", state_path.display());
            }
        }
        Command::Once(args) => {
            let config = config::sync_config(args)?;
            syncer::run_once(&config)?;
        }
        Command::Status(args) => {
            let config = config::status_config(args)?;
            let state = state::State::load(&config.state_path)?;
            let conn = rtkdb::open_read_only(&config.db_path)?;
            let latest_command_id = rtkdb::latest_command_id(&conn)?;
            let pending_count = rtkdb::pending_count(&conn, state.last_synced_id)?;

            println!("Config state:{}", config.state_path.display());
            println!("RTK DB:{}", config.db_path.display());
            println!(
                "Machine ID:{}",
                state.machine_id.as_deref().unwrap_or("<unset>")
            );
            println!("Last synced ID:{}", state.last_synced_id);
            println!(
                "Last synced at:{}",
                state
                    .last_synced_at
                    .map(|value| value.to_rfc3339())
                    .unwrap_or_else(|| "<never>".to_string())
            );
            println!(
                "Latest RTK command ID:{}",
                latest_command_id
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "<none>".to_string())
            );
            println!("Pending events: {pending_count}");
            service::print_status();
        }
        Command::Update(args) => {
            update::install_latest(args)?;
        }
        Command::RunService(args) => {
            syncer::run_daemon(rtk_sync::cli::SyncArgs {
                config: args.config,
                db: None,
                state: None,
                endpoint: None,
                token_env: None,
                machine_id: None,
                batch_size: None,
                allow_insecure_http: false,
                dry_run: false,
            })?;
        }
        Command::InstallService(args) => {
            service::install(args)?;
        }
        Command::UninstallService(args) => {
            service::uninstall(args)?;
        }
    }

    Ok(())
}

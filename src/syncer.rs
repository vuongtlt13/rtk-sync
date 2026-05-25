use crate::cli::SyncArgs;
use crate::client;
use crate::config::{self, SyncConfig};
use crate::machine;
use crate::rtkdb;
use crate::state::State;
use anyhow::{Context, Result};
use chrono::Utc;
use std::thread;
use std::time::Duration;

pub fn run_once(config: &SyncConfig) -> Result<()> {
    log_run_start(config);
    run_batch(config)?;
    println!("rtk-sync: sync completed");
    Ok(())
}

fn run_batch(config: &SyncConfig) -> Result<bool> {
    println!(
        "rtk-sync: loading state from {}",
        config.state_path.display()
    );

    let mut state = State::load(&config.state_path)?;
    let machine_id = machine::resolve_machine_id(config.machine_id.as_deref(), &mut state)?;
    println!(
        "rtk-sync: machine_id={} checkpoint={}",
        machine_id, state.last_synced_id
    );

    if !config.dry_run
        && state.machine_id.as_deref() == Some(&machine_id)
        && !config.state_path.exists()
    {
        println!("rtk-sync: initializing state file");
        state.save(&config.state_path)?;
    }

    println!(
        "rtk-sync: opening RTK DB read-only: {}",
        config.db_path.display()
    );
    let conn = rtkdb::open_read_only(&config.db_path)?;

    println!(
        "rtk-sync: fetching up to {} events after local_id {}",
        config.batch_size, state.last_synced_id
    );
    let events = rtkdb::fetch_events(&conn, state.last_synced_id, config.batch_size, &machine_id)?;
    if events.is_empty() {
        println!("rtk-sync: no events to sync");
        return Ok(false);
    }

    let first_id = events
        .first()
        .map(|event| event.local_id)
        .unwrap_or_default();
    let last_id = events
        .last()
        .map(|event| event.local_id)
        .unwrap_or_default();
    if config.dry_run {
        println!(
            "rtk-sync: dry run fetched {} events (local_id {}..={}); no upload or checkpoint update",
            events.len(),
            first_id,
            last_id
        );
        for event in &events {
            let payload =
                serde_json::to_string(event).context("failed to serialize dry-run event")?;
            println!("rtk-sync: dry-run event {payload}");
        }
        return Ok(false);
    }

    println!(
        "rtk-sync: uploading {} events to {} (local_id {}..={})",
        events.len(),
        config.endpoint,
        first_id,
        last_id
    );

    let previous_checkpoint = state.last_synced_id;
    let result = client::upload_events(&config.endpoint, &config.token, &machine_id, &events)
        .context("failed to upload events")?;
    println!(
        "rtk-sync: upload accepted={} duplicates={} server_max_local_id={}",
        result.accepted, result.duplicates, result.max_local_id
    );

    if result.max_local_id <= previous_checkpoint {
        anyhow::bail!(
            "server max_local_id {} did not advance past checkpoint {}",
            result.max_local_id,
            previous_checkpoint
        );
    }

    state.last_synced_id = result.max_local_id;
    state.last_synced_at = Some(Utc::now());
    state.save(&config.state_path)?;

    println!(
        "rtk-sync: checkpoint updated to {} in {}",
        result.max_local_id,
        config.state_path.display()
    );
    Ok(events.len() == config.batch_size)
}

fn log_run_start(config: &SyncConfig) {
    let started_at = Utc::now();
    println!("rtk-sync: sync started at {}", started_at.to_rfc3339());
    println!(
        "rtk-sync: endpoint={} token={}",
        display_endpoint(&config.endpoint),
        mask_token(&config.token)
    );
}

fn display_endpoint(endpoint: &str) -> &str {
    if endpoint.is_empty() {
        "<unset>"
    } else {
        endpoint
    }
}

fn mask_token(token: &str) -> String {
    if token.is_empty() {
        return "<unset>".to_string();
    }

    let chars = token.chars().collect::<Vec<_>>();
    if chars.len() <= 8 {
        return "***".to_string();
    }

    let prefix = chars.iter().take(4).collect::<String>();
    let suffix = chars.iter().skip(chars.len() - 4).collect::<String>();
    format!("{prefix}***{suffix}")
}

pub fn run_service_interval(config: &SyncConfig) {
    log_run_start(config);
    loop {
        match run_batch(config) {
            Ok(true) => println!("rtk-sync: batch full; checking for more events"),
            Ok(false) => break,
            Err(error) => {
                eprintln!("rtk-sync: sync failed: {error:#}");
                break;
            }
        }
    }
    println!("rtk-sync: sync interval completed");
}

pub fn run_daemon(args: SyncArgs) -> Result<()> {
    println!("rtk-sync: service loop started");
    let mut next_interval = config::DEFAULT_INTERVAL_SECONDS;
    loop {
        next_interval = run_daemon_iteration(&args, next_interval, run_service_interval);
        println!("rtk-sync: sleeping for{}s", next_interval);
        thread::sleep(Duration::from_secs(next_interval));
    }
}

pub fn run_daemon_iteration(
    args: &SyncArgs,
    fallback_interval: u64,
    run_interval: impl FnOnce(&SyncConfig),
) -> u64 {
    match config::sync_config(args.clone()) {
        Ok(config) => {
            let interval = config.interval;
            run_interval(&config);
            interval
        }
        Err(error) => {
            eprintln!("rtk-sync: failed to reload config: {error:#}");
            fallback_interval
        }
    }
}

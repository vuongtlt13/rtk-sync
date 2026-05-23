use crate::client;
use crate::config::SyncConfig;
use crate::machine;
use crate::rtkdb;
use crate::state::State;
use anyhow::{Context, Result};
use chrono::Utc;
use std::thread;
use std::time::Duration;

pub fn run_once(config: &SyncConfig) -> Result<()> {
    let started_at = Utc::now();
    println!("rtk-sync: sync started at {}", started_at.to_rfc3339());
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
        return Ok(());
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
        return Ok(());
    }

    println!(
        "rtk-sync: uploading {} events to {} (local_id {}..={})",
        events.len(),
        config.endpoint,
        first_id,
        last_id
    );

    let result = client::upload_events(&config.endpoint, &config.token, &machine_id, &events)
        .context("failed to upload events")?;
    println!(
        "rtk-sync: upload accepted={} duplicates={} server_max_local_id={}",
        result.accepted, result.duplicates, result.max_local_id
    );

    state.last_synced_id = result.max_local_id;
    state.last_synced_at = Some(Utc::now());
    state.save(&config.state_path)?;

    println!(
        "rtk-sync: checkpoint updated to {} in {}",
        result.max_local_id,
        config.state_path.display()
    );
    println!("rtk-sync: sync completed");
    Ok(())
}

pub fn run_daemon(config: SyncConfig, interval_seconds: u64) -> Result<()> {
    println!(
        "rtk-sync: service loop started interval={}s",
        interval_seconds
    );
    loop {
        if let Err(error) = run_once(&config) {
            eprintln!("rtk-sync: sync failed: {error:#}");
        }
        println!("rtk-sync: sleeping for {}s", interval_seconds);
        thread::sleep(Duration::from_secs(interval_seconds));
    }
}

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct RtkEvent {
    pub source_id: String,
    pub machine_id: String,
    pub local_id: i64,
    pub command: String,
    pub original_cmd: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub saved_tokens: i64,
    pub savings_pct: f64,
    pub exec_time_ms: i64,
    pub project_path: String,
    pub created_at: DateTime<Utc>,
}

pub fn open_read_only(db_path: &Path) -> Result<Connection> {
    Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY).with_context(|| {
        format!(
            "failed to open RTK database read-only: {}",
            db_path.display()
        )
    })
}

pub fn inspect(db_path: &Path) -> Result<()> {
    let conn = open_read_only(db_path)?;
    println!("DB: {}", db_path.display());
    println!("Tables:");

    let tables = list_tables(&conn)?;
    for table in &tables {
        println!("- {table}");
    }

    if tables.iter().any(|table| table == "commands") {
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM commands", [], |row| row.get(0))
            .context("failed to count commands")?;
        let latest: Option<(i64, String)> = conn
            .query_row(
                "SELECT id, timestamp FROM commands ORDER BY id DESC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .context("failed to read latest command")?;
        println!("Commands: {count}");
        if let Some((id, timestamp)) = latest {
            println!("Latest command: id={id} timestamp={timestamp}");
        }
        println!("commands schema:");
        for column in table_info(&conn, "commands")? {
            println!("- {} {}", column.name, column.column_type);
        }
    } else {
        println!("commands table: missing");
    }

    if tables.iter().any(|table| table == "parse_failures") {
        let failures: i64 = conn
            .query_row("SELECT COUNT(*) FROM parse_failures", [], |row| row.get(0))
            .context("failed to count parse failures")?;
        println!("Parse failures: {failures}");
    }

    Ok(())
}

pub fn fetch_events(
    conn: &Connection,
    after_id: i64,
    limit: usize,
    machine_id: &str,
) -> Result<Vec<RtkEvent>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, timestamp, original_cmd, rtk_cmd, input_tokens, output_tokens, saved_tokens, savings_pct, COALESCE(exec_time_ms, 0), COALESCE(project_path, '')
             FROM commands
             WHERE id > ?1
             ORDER BY id ASC
             LIMIT ?2",
        )
        .context("failed to prepare command fetch query")?;

    let rows = stmt
        .query_map(params![after_id, limit as i64], |row| {
            let local_id: i64 = row.get(0)?;
            let timestamp: String = row.get(1)?;
            let created_at = parse_timestamp(&timestamp).map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(
                    1,
                    rusqlite::types::Type::Text,
                    Box::new(err),
                )
            })?;

            Ok(RtkEvent {
                source_id: format!("{machine_id}:{local_id}"),
                machine_id: machine_id.to_string(),
                local_id,
                original_cmd: row.get(2)?,
                command: row.get(3)?,
                input_tokens: row.get(4)?,
                output_tokens: row.get(5)?,
                saved_tokens: row.get(6)?,
                savings_pct: row.get(7)?,
                exec_time_ms: row.get(8)?,
                project_path: row.get(9)?,
                created_at,
            })
        })
        .context("failed to query commands")?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to map commands")
}

fn parse_timestamp(timestamp: &str) -> std::result::Result<DateTime<Utc>, chrono::ParseError> {
    Ok(DateTime::parse_from_rfc3339(timestamp)?.with_timezone(&Utc))
}

fn list_tables(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .context("failed to prepare table list query")?;
    let rows = stmt
        .query_map([], |row| row.get(0))
        .context("failed to query table list")?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to read table list")
}

#[derive(Debug)]
struct ColumnInfo {
    name: String,
    column_type: String,
}

fn table_info(conn: &Connection, table: &str) -> Result<Vec<ColumnInfo>> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .with_context(|| format!("failed to prepare schema query for {table}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ColumnInfo {
                name: row.get(1)?,
                column_type: row.get(2)?,
            })
        })
        .with_context(|| format!("failed to query schema for {table}"))?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("failed to read schema for {table}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_command_rows_to_events() {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        create_commands_table(&conn);
        conn.execute(
            "INSERT INTO commands (id, timestamp, original_cmd, rtk_cmd, input_tokens, output_tokens, saved_tokens, savings_pct, exec_time_ms, project_path)
             VALUES (7, '2026-05-23T10:30:00Z', 'git status', 'rtk git status', 100, 25, 75, 75.0, 12, '/repo')",
            [],
        )
        .expect("insert command");

        let events = fetch_events(&conn, 0, 100, "machine-1").expect("fetch events");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].source_id, "machine-1:7");
        assert_eq!(events[0].command, "rtk git status");
        assert_eq!(events[0].original_cmd, "git status");
        assert_eq!(events[0].saved_tokens, 75);
    }

    fn create_commands_table(conn: &Connection) {
        conn.execute(
            "CREATE TABLE commands (
                id INTEGER PRIMARY KEY,
                timestamp TEXT NOT NULL,
                original_cmd TEXT NOT NULL,
                rtk_cmd TEXT NOT NULL,
                input_tokens INTEGER NOT NULL,
                output_tokens INTEGER NOT NULL,
                saved_tokens INTEGER NOT NULL,
                savings_pct REAL NOT NULL,
                exec_time_ms INTEGER DEFAULT 0,
                project_path TEXT DEFAULT ''
            )",
            [],
        )
        .expect("create commands table");
    }
}

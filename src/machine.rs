use crate::state::State;
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::env;

pub fn resolve_machine_id(cli_machine_id: Option<&str>, state: &mut State) -> Result<String> {
    if let Some(machine_id) = cli_machine_id {
        state.machine_id = Some(machine_id.to_string());
        return Ok(machine_id.to_string());
    }

    if let Some(machine_id) = &state.machine_id {
        return Ok(machine_id.clone());
    }

    let machine_id = generate_machine_id()?;
    state.machine_id = Some(machine_id.clone());
    Ok(machine_id)
}

fn generate_machine_id() -> Result<String> {
    let hostname = hostname::get()
        .context("failed to read hostname")?
        .to_string_lossy()
        .to_string();
    let username = env::var("USER")
        .or_else(|_| env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string());
    let os = env::consts::OS;
    let arch = env::consts::ARCH;
    let fingerprint = format!("{hostname}|{username}|{os}|{arch}");
    let hash = Sha256::digest(fingerprint.as_bytes());
    let suffix = hash[..4]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(format!("{}-{suffix}", sanitize_hostname(&hostname)))
}

fn sanitize_hostname(hostname: &str) -> String {
    let safe_hostname = hostname
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();

    if safe_hostname.is_empty() {
        "machine".to_string()
    } else {
        safe_hostname
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_machine_id_wins() {
        let mut state = State {
            machine_id: Some("old".to_string()),
            ..State::default()
        };
        let machine_id = resolve_machine_id(Some("new"), &mut state).expect("resolve machine id");
        assert_eq!(machine_id, "new");
        assert_eq!(state.machine_id.as_deref(), Some("new"));
    }

    #[test]
    fn state_machine_id_is_reused() {
        let mut state = State {
            machine_id: Some("existing".to_string()),
            ..State::default()
        };
        let machine_id = resolve_machine_id(None, &mut state).expect("resolve machine id");
        assert_eq!(machine_id, "existing");
    }

    #[test]
    fn sanitizes_hostname() {
        assert_eq!(sanitize_hostname("MacBook Pro.local"), "MacBook-Pro-local");
        assert_eq!(sanitize_hostname("..."), "machine");
    }
}

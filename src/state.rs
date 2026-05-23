use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct State {
    pub machine_id: Option<String>,
    pub last_synced_id: i64,
    pub last_synced_at: Option<DateTime<Utc>>,
}

impl State {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read state file: {}", path.display()))?;
        serde_json::from_str(&content)
            .with_context(|| format!("failed to parse state file: {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create state directory: {}", parent.display())
            })?;
        }

        let tmp_path = path.with_extension("json.tmp");
        let content = serde_json::to_string_pretty(self).context("failed to serialize state")?;
        fs::write(&tmp_path, content)
            .with_context(|| format!("failed to write temp state file: {}", tmp_path.display()))?;
        fs::rename(&tmp_path, path)
            .with_context(|| format!("failed to replace state file: {}", path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_state_is_default() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let state = State::load(&dir.path().join("state.json")).expect("load missing state");
        assert_eq!(state.last_synced_id, 0);
        assert!(state.machine_id.is_none());
    }

    #[test]
    fn saves_and_loads_state() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let path = dir.path().join("state.json");
        let state = State {
            machine_id: Some("machine-1".to_string()),
            last_synced_id: 42,
            last_synced_at: Some(Utc::now()),
        };

        state.save(&path).expect("save state");
        let loaded = State::load(&path).expect("load state");

        assert_eq!(loaded.machine_id.as_deref(), Some("machine-1"));
        assert_eq!(loaded.last_synced_id, 42);
        assert!(loaded.last_synced_at.is_some());
    }
}

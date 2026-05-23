use crate::rtkdb::RtkEvent;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize)]
struct UploadRequest<'a> {
    machine_id: &'a str,
    events: &'a [RtkEvent],
}

#[derive(Debug, Clone, Deserialize)]
pub struct UploadResult {
    pub accepted: usize,
    pub duplicates: usize,
    pub max_local_id: i64,
}

pub fn upload_events(
    endpoint: &str,
    token: &str,
    machine_id: &str,
    events: &[RtkEvent],
) -> Result<UploadResult> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(30))
        .try_proxy_from_env(false)
        .build();
    let request = UploadRequest { machine_id, events };
    let body = serde_json::to_value(&request).context("failed to serialize upload request")?;
    let response = agent
        .post(endpoint)
        .set("Authorization", &format!("Bearer {token}"))
        .set("Content-Type", "application/json")
        .send_json(body);

    match response {
        Ok(response) => response
            .into_json::<UploadResult>()
            .context("failed to parse upload response"),
        Err(ureq::Error::Status(code, response)) => {
            let message = response
                .into_string()
                .unwrap_or_else(|_| "<unreadable response body>".to_string());
            Err(anyhow!("upload failed with status {code}: {message}"))
        }
        Err(err) => Err(anyhow!("upload failed: {err}")),
    }
}

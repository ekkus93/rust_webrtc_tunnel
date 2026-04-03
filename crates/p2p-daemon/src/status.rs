use std::fs;
use std::path::PathBuf;

use p2p_core::{AppConfig, DaemonState, NodeRole, PeerId, SessionId};
use serde::Serialize;

use crate::DaemonError;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DaemonStatus {
    pub peer_id: PeerId,
    pub role: NodeRole,
    pub mqtt_connected: bool,
    pub active_session_id: Option<String>,
    pub current_state: DaemonState,
}

impl DaemonStatus {
    pub fn new(
        peer_id: PeerId,
        role: NodeRole,
        mqtt_connected: bool,
        active_session_id: Option<SessionId>,
        current_state: DaemonState,
    ) -> Self {
        Self {
            peer_id,
            role,
            mqtt_connected,
            active_session_id: active_session_id.map(|id| id.to_string()),
            current_state,
        }
    }
}

pub struct StatusWriter {
    enabled: bool,
    path: PathBuf,
}

impl StatusWriter {
    pub fn new(config: &AppConfig) -> Self {
        Self { enabled: config.health.write_status_file, path: config.health.status_file.clone() }
    }

    pub async fn write(&self, status: DaemonStatus) -> Result<(), DaemonError> {
        if !self.enabled {
            return Ok(());
        }
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_vec_pretty(&status)
            .map_err(|error| DaemonError::Logging(error.to_string()))?;
        tokio::fs::write(&self.path, json).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use p2p_core::{DaemonState, NodeRole};

    use super::{DaemonStatus, StatusWriter};

    #[tokio::test]
    async fn writes_status_json_without_secrets() {
        let temp_path =
            std::env::temp_dir().join(format!("p2ptunnel-status-{}.json", std::process::id()));
        let writer = StatusWriter { enabled: true, path: temp_path.clone() };
        writer
            .write(DaemonStatus::new(
                "offer-home".parse().expect("peer id"),
                NodeRole::Offer,
                true,
                Some(p2p_core::SessionId::new([7_u8; 16])),
                DaemonState::Idle,
            ))
            .await
            .expect("status file should write");
        let content = tokio::fs::read_to_string(&temp_path).await.expect("status file should read");
        assert!(content.contains("\"peer_id\""));
        assert!(!content.contains("private"));
        let _ = tokio::fs::remove_file(PathBuf::from(&temp_path)).await;
    }
}

//! Forward routing: the runtime [`ForwardTable`] built from the configured
//! [`ForwardRule`]s, the lookup result/error types, and the forward-id /
//! listen-host validators.

use crate::error::ConfigError;
use crate::ids::PeerId;

use super::ForwardRule;
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OfferForwardBind {
    pub forward_id: String,
    pub listen_host: String,
    pub listen_port: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetAddr {
    pub host: String,
    pub port: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ForwardLookupError {
    UnknownForward,
    ForbiddenForward,
    MissingOfferConfig,
    MissingAnswerConfig,
}

#[derive(Clone, Debug)]
pub struct ForwardTable {
    by_id: std::collections::HashMap<String, ForwardRule>,
}

impl ForwardTable {
    pub fn new(forwards: &[ForwardRule]) -> Self {
        Self {
            by_id: forwards.iter().map(|forward| (forward.id.clone(), forward.clone())).collect(),
        }
    }

    pub fn get(&self, forward_id: &str) -> Option<&ForwardRule> {
        self.by_id.get(forward_id)
    }

    pub fn offer_listeners(&self) -> Result<Vec<OfferForwardBind>, ForwardLookupError> {
        let mut listeners = Vec::new();
        for forward in self.by_id.values() {
            let offer = forward.offer.as_ref().ok_or(ForwardLookupError::MissingOfferConfig)?;
            listeners.push(OfferForwardBind {
                forward_id: forward.id.clone(),
                listen_host: offer.listen_host.clone(),
                listen_port: offer.listen_port,
            });
        }
        listeners.sort_by(|left, right| left.forward_id.cmp(&right.forward_id));
        Ok(listeners)
    }

    pub fn target_for(
        &self,
        forward_id: &str,
        remote_peer_id: &PeerId,
    ) -> Result<TargetAddr, ForwardLookupError> {
        let forward = self.by_id.get(forward_id).ok_or(ForwardLookupError::UnknownForward)?;
        let answer = forward.answer.as_ref().ok_or(ForwardLookupError::MissingAnswerConfig)?;
        if !answer.allow_remote_peers.contains(remote_peer_id) {
            return Err(ForwardLookupError::ForbiddenForward);
        }
        Ok(TargetAddr { host: answer.target_host.clone(), port: answer.target_port })
    }
}

pub(crate) fn validate_forward_id(id: &str) -> Result<(), ConfigError> {
    if id.is_empty() {
        return Err(ConfigError::InvalidConfig("forward id must not be empty".to_owned()));
    }
    if id.len() > 64 {
        return Err(ConfigError::InvalidConfig(format!("forward id '{id}' exceeds 64 characters")));
    }
    if !id.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')) {
        return Err(ConfigError::InvalidConfig(format!(
            "forward id '{id}' contains invalid characters"
        )));
    }
    Ok(())
}

pub(crate) fn validate_listen_host(host: &str, forward_id: &str) -> Result<(), ConfigError> {
    if host.is_empty() {
        return Err(ConfigError::InvalidConfig(format!(
            "forward '{forward_id}' listen_host must be set"
        )));
    }
    Ok(())
}

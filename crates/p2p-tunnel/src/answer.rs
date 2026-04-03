use p2p_core::TunnelAnswerConfig;
use tokio::net::TcpStream;

use crate::TunnelError;

#[derive(Clone, Debug)]
pub struct AnswerTargetConnector {
    config: TunnelAnswerConfig,
}

impl AnswerTargetConnector {
    pub fn new(config: &TunnelAnswerConfig) -> Self {
        Self { config: config.clone() }
    }

    pub async fn connect_target(&self) -> Result<TcpStream, TunnelError> {
        TcpStream::connect((self.config.target_host.as_str(), self.config.target_port))
            .await
            .map_err(TunnelError::from)
    }
}

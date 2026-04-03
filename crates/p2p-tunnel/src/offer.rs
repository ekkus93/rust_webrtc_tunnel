use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use p2p_core::TunnelOfferConfig;
use tokio::net::{TcpListener, TcpStream};

use crate::TunnelError;

pub struct OfferListener {
    listener: TcpListener,
    config: TunnelOfferConfig,
    active_client: Arc<AtomicBool>,
}

impl OfferListener {
    pub async fn bind(config: &TunnelOfferConfig) -> Result<Self, TunnelError> {
        let listener = TcpListener::bind((config.listen_host.as_str(), config.listen_port)).await?;
        Ok(Self {
            listener,
            config: config.clone(),
            active_client: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr, TunnelError> {
        Ok(self.listener.local_addr()?)
    }

    pub fn active_client_count(&self) -> usize {
        usize::from(self.active_client.load(Ordering::SeqCst))
    }

    pub async fn accept_client(&self) -> Result<OfferClient, TunnelError> {
        loop {
            let (stream, address) = self.listener.accept().await?;
            if self.active_client_count() >= self.config.max_concurrent_clients {
                tracing::warn!("rejecting extra client from {address} because tunnel is busy");
                drop(stream);
                if self.config.deny_when_busy {
                    continue;
                }
                continue;
            }

            if self
                .active_client
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return Ok(OfferClient {
                    stream: Some(stream),
                    active_client: Arc::clone(&self.active_client),
                });
            }

            tracing::warn!("rejecting extra client from {address} because tunnel is busy");
            drop(stream);
        }
    }
}

pub struct OfferClient {
    stream: Option<TcpStream>,
    active_client: Arc<AtomicBool>,
}

impl OfferClient {
    pub fn into_stream(mut self) -> Result<TcpStream, TunnelError> {
        self.stream.take().ok_or_else(|| {
            TunnelError::InvalidFrame("offer client stream already taken".to_owned())
        })
    }
}

impl Drop for OfferClient {
    fn drop(&mut self) {
        self.active_client.store(false, Ordering::SeqCst);
    }
}

//! Post-DCEP data-plane readiness probe.
//!
//! After the WebRTC data channel reports open (DCEP), the underlying SCTP path can still
//! silently fail to carry application data in one direction (observed on some Android
//! vnet / remote-NAT combinations: the data channel opens, but offer→answer `DATA` is
//! black-holed). [`probe_data_plane`] gates the offer's transition to bridging on an
//! actual application-level round trip: it sends a tunnel `Ping` and requires a matching
//! `Pong` before any real stream `OPEN` is sent, so a black-holed data plane fails fast
//! (with a clear error) instead of leaving the local TCP client hung at zero bytes.

use std::fmt::Write as _;
use std::time::Duration;

use p2p_core::TunnelFrameType;
use p2p_webrtc::{DataChannelEvent, DataChannelHandle};
use rand_core::{OsRng, RngCore};

use crate::{TunnelError, TunnelFrame, TunnelFrameCodec};

/// Length of the random probe nonce, in bytes.
const PROBE_NONCE_LEN: usize = 16;

/// Send a tunnel `Ping` over `data_channel` and wait for the matching `Pong`, bounded by
/// `timeout`.
///
/// The caller MUST be the sole consumer of [`DataChannelHandle::next_event`] for the
/// duration of the probe (the offer session hands the channel to `run_multiplex_offer`
/// only after this resolves). While waiting, the probe answers any inbound `Ping` with a
/// `Pong` so it is well-behaved if both sides probe.
///
/// Returns [`TunnelError::DataPlaneProbeTimeout`] if no matching `Pong` arrives in time,
/// [`TunnelError::DataChannelClosed`] if the channel closes first, and
/// [`TunnelError::DataPlaneProbeFailed`] if an unexpected stream frame arrives before the
/// probe completes.
pub async fn probe_data_plane(
    data_channel: &DataChannelHandle,
    timeout: Duration,
) -> Result<(), TunnelError> {
    let mut nonce = [0u8; PROBE_NONCE_LEN];
    OsRng.fill_bytes(&mut nonce);
    probe_data_plane_with_nonce(data_channel, timeout, &nonce).await
}

/// Inner probe with a caller-supplied nonce, so tests can use a deterministic value.
async fn probe_data_plane_with_nonce(
    data_channel: &DataChannelHandle,
    timeout: Duration,
    nonce: &[u8],
) -> Result<(), TunnelError> {
    let ping = TunnelFrameCodec::encode(&TunnelFrame::ping(nonce.to_vec()))?;
    data_channel.send(&ping).await?;
    tracing::info!(
        target: "tunnel",
        nonce = %redact_nonce(nonce),
        timeout_ms = timeout.as_millis() as u64,
        "data-plane probe sent",
    );

    let outcome = tokio::time::timeout(timeout, await_pong(data_channel, nonce)).await;
    match outcome {
        Ok(result) => result,
        Err(_) => {
            tracing::warn!(
                target: "tunnel",
                timeout_ms = timeout.as_millis() as u64,
                "data-plane probe timed out",
            );
            Err(TunnelError::DataPlaneProbeTimeout(timeout))
        }
    }
}

/// Read data-channel events until the matching `Pong` arrives (or the channel breaks).
async fn await_pong(data_channel: &DataChannelHandle, nonce: &[u8]) -> Result<(), TunnelError> {
    loop {
        match data_channel.next_event().await {
            Some(DataChannelEvent::Message(payload)) => {
                let frame = TunnelFrameCodec::decode(&payload)?;
                match frame.frame_type {
                    TunnelFrameType::Pong => {
                        if frame.payload.as_slice() == nonce {
                            tracing::info!(target: "tunnel", "data-plane probe acknowledged");
                            return Ok(());
                        }
                        tracing::warn!(
                            target: "tunnel",
                            "data-plane probe received mismatched pong; ignoring",
                        );
                    }
                    TunnelFrameType::Ping => {
                        // Answer inbound pings while we wait so a peer probing us in the
                        // opposite direction also succeeds.
                        let pong = TunnelFrameCodec::encode(&TunnelFrame::pong(frame.payload))?;
                        data_channel.send(&pong).await?;
                    }
                    other => {
                        tracing::warn!(
                            target: "tunnel",
                            frame_type = ?other,
                            "data-plane probe received unexpected pre-probe frame",
                        );
                        return Err(TunnelError::DataPlaneProbeFailed(format!(
                            "unexpected {other:?} frame before probe completion"
                        )));
                    }
                }
            }
            Some(DataChannelEvent::Open) => {}
            Some(DataChannelEvent::Closed) | None => {
                return Err(TunnelError::DataChannelClosed);
            }
        }
    }
}

/// A short, hex-encoded prefix of the nonce for log correlation. Never logs the full
/// nonce or any other payload bytes.
fn redact_nonce(nonce: &[u8]) -> String {
    let mut out = String::with_capacity(8);
    for byte in nonce.iter().take(4) {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_nonce_emits_only_short_prefix() {
        let nonce = [0xde, 0xad, 0xbe, 0xef, 0x01, 0x02, 0x03, 0x04];
        assert_eq!(redact_nonce(&nonce), "deadbeef");
    }

    #[test]
    fn redact_nonce_handles_short_input() {
        assert_eq!(redact_nonce(&[0xab, 0xcd]), "abcd");
    }
}

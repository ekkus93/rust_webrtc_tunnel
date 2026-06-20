//! Integration tests for the offer's mid-session data-plane heartbeat, over a real
//! connected WebRTC data-channel pair. The `OfferHeartbeat` state machine is unit-tested
//! in `multiplex/offer.rs`; these exercise the wiring inside `run_multiplex_offer`
//! (periodic `Ping`, `Pong` interception, and the `DataPlaneHeartbeatLost` teardown).

use super::support::*;
use crate::TunnelFrameCodec;
use p2p_webrtc::{DataChannelEvent, DataChannelHandle};

/// Reply with a `Pong` to every inbound `Ping` (echoing the payload), forever.
async fn echo_all_pings(channel: DataChannelHandle) {
    loop {
        match channel.next_event().await {
            Some(DataChannelEvent::Message(bytes)) => {
                if let Ok(frame) = TunnelFrameCodec::decode(&bytes) {
                    if frame.frame_type == TunnelFrameType::Ping {
                        if let Ok(pong) =
                            TunnelFrameCodec::encode(&TunnelFrame::pong(frame.payload))
                        {
                            let _ = channel.send(&pong).await;
                        }
                    }
                }
            }
            Some(_) => {}
            None => return,
        }
    }
}

/// A `TunnelConfig` with a fast heartbeat so loss is detected in ~100ms instead of ~15s.
fn fast_heartbeat_config() -> TunnelConfig {
    let mut tunnel = sample_tunnel_config();
    tunnel.data_plane_heartbeat_interval_ms = 50;
    tunnel.data_plane_heartbeat_max_misses = 2;
    tunnel
}

/// Accept one local client so the offer has an initial (opening) stream that keeps its loop
/// alive. Returns the offer-side stream plus the live client (kept to hold the TCP conn open).
async fn one_local_client() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("local listener binds");
    let addr = listener.local_addr().expect("local addr");
    let client = TcpStream::connect(addr).await.expect("client connects");
    let (offer_stream, _) = listener.accept().await.expect("offer accepts");
    (offer_stream, client)
}

#[tokio::test]
async fn heartbeat_loss_tears_down_offer_when_answer_stops_responding() {
    // The answer channel is never driven, so it never echoes the heartbeat Pongs.
    let (offer_peer, answer_peer, offer_channel, _answer_channel) = connected_channels().await;
    let (offer_stream, _client) = one_local_client().await;
    let (tx, mut rx) = mpsc::channel(1);
    drop(tx); // no further clients

    let tunnel = fast_heartbeat_config();
    let result = timeout(
        Duration::from_secs(5),
        run_multiplex_offer(offer_channel, &tunnel, OfferClient::new("ssh", offer_stream), &mut rx),
    )
    .await
    .expect("offer mux should finish once the heartbeat detects the dead data plane");

    assert!(
        matches!(result, Err(TunnelError::DataPlaneHeartbeatLost { missed }) if missed >= 2),
        "expected DataPlaneHeartbeatLost, got {result:?}",
    );

    offer_peer.close().await.expect("offer peer closes");
    answer_peer.close().await.expect("answer peer closes");
}

#[tokio::test]
async fn heartbeat_stays_alive_while_answer_echoes_pongs() {
    let (offer_peer, answer_peer, offer_channel, answer_channel) = connected_channels().await;
    let echo_task = tokio::spawn(echo_all_pings(answer_channel));

    let (offer_stream, _client) = one_local_client().await;
    // Keep `tx` alive so the accept arm pends and the loop never exits on its own; only a
    // heartbeat loss could end it — and it must not, because pongs keep flowing.
    let (_tx, mut rx) = mpsc::channel(1);

    let tunnel = fast_heartbeat_config();
    let mut offer_task = tokio::spawn(async move {
        run_multiplex_offer(offer_channel, &tunnel, OfferClient::new("ssh", offer_stream), &mut rx)
            .await
    });

    // Span many heartbeat intervals (~50ms each); the echoing answer keeps it alive, so the
    // offer task must still be running.
    assert!(
        timeout(Duration::from_millis(400), &mut offer_task).await.is_err(),
        "heartbeat must not tear down a session whose answer keeps echoing pongs",
    );

    offer_task.abort();
    echo_task.abort();
    offer_peer.close().await.expect("offer peer closes");
    answer_peer.close().await.expect("answer peer closes");
}

//! End-to-end tests for the post-DCEP data-plane probe over a real connected
//! WebRTC data-channel pair.

use super::support::*;
use crate::{TunnelFrame, TunnelFrameCodec, probe_data_plane};
use p2p_webrtc::{DataChannelEvent, DataChannelHandle};

/// Drive `answer_channel`, replying to the first inbound `Ping` with the given
/// `pong_payload` (or a faithful echo when `echo` is true). Returns after one reply.
async fn answer_one_ping(channel: DataChannelHandle, echo: bool, pong_payload: Vec<u8>) {
    loop {
        match channel.next_event().await {
            Some(DataChannelEvent::Message(bytes)) => {
                let frame = TunnelFrameCodec::decode(&bytes).expect("answer decodes frame");
                if frame.frame_type == TunnelFrameType::Ping {
                    let payload = if echo { frame.payload } else { pong_payload };
                    let pong =
                        TunnelFrameCodec::encode(&TunnelFrame::pong(payload)).expect("encode pong");
                    channel.send(&pong).await.expect("answer sends pong");
                    return;
                }
            }
            Some(_) => {}
            None => return,
        }
    }
}

#[tokio::test]
async fn probe_succeeds_when_answer_echoes_pong() {
    let (offer_peer, answer_peer, offer_channel, answer_channel) = connected_channels().await;

    // The real answer echoes the ping payload back as the pong payload.
    let answer_task = tokio::spawn(answer_one_ping(answer_channel, true, Vec::new()));

    probe_data_plane(&offer_channel, Duration::from_secs(10))
        .await
        .expect("probe should succeed when the data plane round-trips");

    timeout(Duration::from_secs(5), answer_task)
        .await
        .expect("answer task finishes")
        .expect("join");
    offer_peer.close().await.expect("offer peer closes");
    answer_peer.close().await.expect("answer peer closes");
}

#[tokio::test]
async fn probe_times_out_when_no_pong_arrives() {
    // Keep the peers/channels alive (so the channel does not close) but never reply.
    let (_offer_peer, _answer_peer, offer_channel, _answer_channel) = connected_channels().await;

    let error = probe_data_plane(&offer_channel, Duration::from_millis(250))
        .await
        .expect_err("probe must fail when no pong arrives");

    assert!(
        matches!(error, TunnelError::DataPlaneProbeTimeout(_)),
        "expected a timeout, got {error:?}",
    );
}

/// Receive (and decode) the inbound `Ping` — proving offer→answer delivery works — but
/// deliberately never reply, modeling a one-way data plane. Returns whether the ping arrived.
async fn receive_ping_without_replying(channel: DataChannelHandle) -> bool {
    loop {
        match channel.next_event().await {
            Some(DataChannelEvent::Message(bytes)) => {
                let frame = TunnelFrameCodec::decode(&bytes).expect("answer decodes frame");
                if frame.frame_type == TunnelFrameType::Ping {
                    return true;
                }
            }
            Some(_) => {}
            None => return false,
        }
    }
}

#[tokio::test]
async fn probe_fails_when_delivery_is_one_way_only() {
    // Offer→answer delivery succeeds, but no pong returns: the probe must still fail so user
    // forwarding never starts on a one-way data plane.
    let (offer_peer, answer_peer, offer_channel, answer_channel) = connected_channels().await;

    let answer_task = tokio::spawn(receive_ping_without_replying(answer_channel));

    let error = probe_data_plane(&offer_channel, Duration::from_millis(500))
        .await
        .expect_err("a one-way-only data plane must fail the probe");
    assert!(
        matches!(error, TunnelError::DataPlaneProbeTimeout(_)),
        "expected a timeout, got {error:?}",
    );

    let delivered = timeout(Duration::from_secs(5), answer_task)
        .await
        .expect("answer task finishes")
        .expect("join");
    assert!(delivered, "the offer→answer ping should have been delivered");
    offer_peer.close().await.expect("offer peer closes");
    answer_peer.close().await.expect("answer peer closes");
}

#[tokio::test]
async fn probe_ignores_mismatched_pong_and_times_out() {
    let (offer_peer, answer_peer, offer_channel, answer_channel) = connected_channels().await;

    // Reply with a pong carrying the WRONG payload; the probe must not accept it.
    let answer_task = tokio::spawn(answer_one_ping(answer_channel, false, vec![0xaa, 0xbb, 0xcc]));

    let error = probe_data_plane(&offer_channel, Duration::from_millis(500))
        .await
        .expect_err("a mismatched pong must not satisfy the probe");
    assert!(
        matches!(error, TunnelError::DataPlaneProbeTimeout(_)),
        "expected a timeout, got {error:?}",
    );

    timeout(Duration::from_secs(5), answer_task)
        .await
        .expect("answer task finishes")
        .expect("join");
    offer_peer.close().await.expect("offer peer closes");
    answer_peer.close().await.expect("answer peer closes");
}

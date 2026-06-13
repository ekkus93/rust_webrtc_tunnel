//! Signaling-trace decoding: decode recorded transport payloads back into typed
//! records and assert on message-type provenance (e.g. that the answer side never
//! initiates fresh-session or reconnect signaling).

use p2p_core::MessageType;
use p2p_signaling::{ReplayCache, SignalCodec};

#[derive(Clone)]
pub(crate) struct DecodedSignalRecord {
    pub(crate) session_id: p2p_core::SessionId,
    pub(crate) sender_peer_id: p2p_core::PeerId,
    pub(crate) message_type: MessageType,
}

pub(crate) fn decode_signal_records(
    payloads: &[Vec<u8>],
    codec: &SignalCodec<'_>,
) -> Vec<DecodedSignalRecord> {
    payloads
        .iter()
        .map(|payload| {
            let mut replay_cache = ReplayCache::new(64);
            let (_envelope, message, _sender) = codec
                .decode(payload, &mut replay_cache, None)
                .expect("recorded signaling payload should decode");
            DecodedSignalRecord {
                session_id: message.session_id,
                sender_peer_id: message.sender_peer_id,
                message_type: message.message_type,
            }
        })
        .collect()
}

pub(crate) fn count_records_from(
    records: &[DecodedSignalRecord],
    sender_peer_id: &str,
    message_type: MessageType,
) -> usize {
    records
        .iter()
        .filter(|record| {
            record.sender_peer_id.as_str() == sender_peer_id && record.message_type == message_type
        })
        .count()
}

pub(crate) fn assert_answer_trace_is_passive(records: &[DecodedSignalRecord]) {
    assert!(
        !records.iter().any(|record| matches!(
            record.message_type,
            MessageType::Offer | MessageType::IceRestartRequest | MessageType::RenegotiateRequest
        )),
        "answer side must not initiate fresh-session or reconnect signaling"
    );
}

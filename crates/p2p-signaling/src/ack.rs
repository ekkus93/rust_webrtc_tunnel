use std::collections::HashMap;

use p2p_core::{ACK_RETRY_LIMIT, ACK_RETRY_TIMEOUT_SECS, MessageType, MsgId};

#[derive(Clone, Debug)]
pub struct PendingAck {
    pub payload: Vec<u8>,
    pub sent_at_ms: u64,
    pub retries: u8,
}

#[derive(Debug, Default)]
pub struct AckTracker {
    pending: HashMap<MsgId, PendingAck>,
}

impl AckTracker {
    pub fn register(
        &mut self,
        msg_id: MsgId,
        message_type: MessageType,
        payload: Vec<u8>,
        sent_at_ms: u64,
    ) {
        if !message_type.requires_ack() {
            return;
        }
        self.pending.insert(msg_id, PendingAck { payload, sent_at_ms, retries: 0 });
    }

    pub fn acknowledge(&mut self, msg_id: &MsgId) -> Option<PendingAck> {
        self.pending.remove(msg_id)
    }

    pub fn retry_due(&mut self, now_ms: u64) -> Vec<(MsgId, Vec<u8>)> {
        let retry_timeout_ms = ACK_RETRY_TIMEOUT_SECS * 1_000;
        self.pending
            .iter_mut()
            .filter_map(|(msg_id, pending)| {
                if pending.retries >= ACK_RETRY_LIMIT {
                    return None;
                }
                if pending.sent_at_ms.saturating_add(retry_timeout_ms) > now_ms {
                    return None;
                }

                pending.retries += 1;
                pending.sent_at_ms = now_ms;
                Some((*msg_id, pending.payload.clone()))
            })
            .collect()
    }

    pub fn expired(&self) -> Vec<MsgId> {
        self.pending
            .iter()
            .filter_map(|(msg_id, pending)| (pending.retries >= ACK_RETRY_LIMIT).then_some(*msg_id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use p2p_core::{ACK_RETRY_LIMIT, ACK_RETRY_TIMEOUT_SECS, MessageType, MsgId};

    use super::AckTracker;

    #[test]
    fn acknowledge_clears_only_matching_pending_message() {
        let mut tracker = AckTracker::default();
        let first = MsgId::new([1_u8; 16]);
        let second = MsgId::new([2_u8; 16]);

        tracker.register(first, MessageType::Offer, vec![1_u8], 0);
        tracker.register(second, MessageType::Answer, vec![2_u8], 0);

        let removed = tracker.acknowledge(&first).expect("matching pending ack should be removed");
        assert_eq!(removed.payload, vec![1_u8]);
        assert!(tracker.acknowledge(&first).is_none(), "removed ack should stay cleared");

        let due = tracker.retry_due(ACK_RETRY_TIMEOUT_SECS * 1_000);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].0, second);
    }

    #[test]
    fn non_ack_required_messages_are_not_tracked() {
        let mut tracker = AckTracker::default();
        let msg_id = MsgId::new([9_u8; 16]);

        tracker.register(msg_id, MessageType::Hello, vec![7_u8], 0);

        assert!(tracker.acknowledge(&msg_id).is_none());
        assert!(tracker.retry_due(u64::MAX).is_empty());
        assert!(tracker.expired().is_empty());
    }

    #[test]
    fn pending_ack_expires_after_retry_limit() {
        let mut tracker = AckTracker::default();
        let msg_id = MsgId::new([3_u8; 16]);
        let retry_timeout_ms = ACK_RETRY_TIMEOUT_SECS * 1_000;

        tracker.register(msg_id, MessageType::IceCandidate, vec![4_u8], 0);

        for attempt in 1..=u64::from(ACK_RETRY_LIMIT) {
            let due = tracker.retry_due(retry_timeout_ms * attempt);
            assert_eq!(due.len(), 1, "retry {attempt} should still be due");
        }

        assert_eq!(tracker.expired(), vec![msg_id]);
        assert!(tracker.retry_due(retry_timeout_ms * (u64::from(ACK_RETRY_LIMIT) + 1)).is_empty());
    }
}

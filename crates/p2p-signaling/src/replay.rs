use std::collections::{HashMap, VecDeque};

use p2p_core::{Kid, MsgId, SessionId};

use crate::error::SignalingError;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
struct ReplayKey {
    sender_kid: Kid,
    msg_id: MsgId,
}

#[derive(Clone, Copy, Debug)]
struct ReplayEntry {
    session_id: SessionId,
    timestamp_ms: u64,
}

#[derive(Debug)]
pub struct ReplayCache {
    entries: HashMap<ReplayKey, ReplayEntry>,
    order: VecDeque<(ReplayKey, u64)>,
    capacity: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplayStatus {
    Fresh,
    DuplicateSameSession,
    DuplicateDifferentSession,
}

#[derive(Clone, Copy, Debug)]
pub struct ReplayCheck {
    pub session_id: SessionId,
    pub timestamp_ms: u64,
    pub now_ms: u64,
    pub max_clock_skew_secs: u64,
    pub max_message_age_secs: u64,
    pub expected_session: Option<SessionId>,
}

impl ReplayCache {
    pub fn new(capacity: usize) -> Self {
        Self { entries: HashMap::new(), order: VecDeque::new(), capacity }
    }

    pub fn check_and_record(
        &mut self,
        sender_kid: Kid,
        msg_id: MsgId,
        check: ReplayCheck,
    ) -> Result<(), SignalingError> {
        match self.check_and_record_status(sender_kid, msg_id, check)? {
            ReplayStatus::Fresh => Ok(()),
            ReplayStatus::DuplicateSameSession => {
                Err(SignalingError::Protocol("duplicate message detected".to_owned()))
            }
            ReplayStatus::DuplicateDifferentSession => Err(SignalingError::Protocol(
                "duplicate msg_id received for a different session".to_owned(),
            )),
        }
    }

    pub fn check_and_record_status(
        &mut self,
        sender_kid: Kid,
        msg_id: MsgId,
        check: ReplayCheck,
    ) -> Result<ReplayStatus, SignalingError> {
        let max_clock_skew_ms = check.max_clock_skew_secs.saturating_mul(1_000);
        let max_message_age_ms = check.max_message_age_secs.saturating_mul(1_000);
        if check.timestamp_ms.saturating_add(max_message_age_ms) < check.now_ms {
            return Err(SignalingError::Protocol("message is too old".to_owned()));
        }
        if check.timestamp_ms > check.now_ms.saturating_add(max_clock_skew_ms) {
            return Err(SignalingError::Protocol(
                "message timestamp is too far in the future".to_owned(),
            ));
        }
        if let Some(expected_session) = check.expected_session
            && expected_session != check.session_id
        {
            return Err(SignalingError::Protocol(
                "message session does not match the active session".to_owned(),
            ));
        }

        let key = ReplayKey { sender_kid, msg_id };
        if let Some(existing) = self.entries.get(&key) {
            if existing.session_id == check.session_id {
                return Ok(ReplayStatus::DuplicateSameSession);
            }
            return Ok(ReplayStatus::DuplicateDifferentSession);
        }

        self.entries.insert(
            key,
            ReplayEntry { session_id: check.session_id, timestamp_ms: check.timestamp_ms },
        );
        self.order.push_back((key, check.timestamp_ms));
        self.prune(check.now_ms, max_message_age_ms);
        Ok(ReplayStatus::Fresh)
    }

    fn prune(&mut self, now_ms: u64, max_message_age_ms: u64) {
        while self.entries.len() > self.capacity {
            if let Some((key, _)) = self.order.pop_front() {
                self.entries.remove(&key);
            }
        }

        while let Some((key, recorded_at)) = self.order.front().copied() {
            if recorded_at.saturating_add(max_message_age_ms) >= now_ms {
                break;
            }
            self.order.pop_front();
            if self.entries.get(&key).is_some_and(|entry| entry.timestamp_ms == recorded_at) {
                self.entries.remove(&key);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use p2p_core::{Kid, MsgId, SessionId};

    use super::{ReplayCache, ReplayCheck, ReplayEntry, ReplayKey, ReplayStatus};
    use crate::error::SignalingError;

    fn kid(seed: u8) -> Kid {
        Kid::new([seed; 32])
    }

    fn msg_id(seed: u8) -> MsgId {
        MsgId::new([seed; 16])
    }

    fn session(seed: u8) -> SessionId {
        SessionId::new([seed; 16])
    }

    fn check(session_id: SessionId, timestamp_ms: u64, now_ms: u64) -> ReplayCheck {
        ReplayCheck {
            session_id,
            timestamp_ms,
            now_ms,
            max_clock_skew_secs: 120,
            max_message_age_secs: 300,
            expected_session: None,
        }
    }

    #[test]
    fn duplicate_within_window_is_rejected_as_replay() {
        let mut cache = ReplayCache::new(4);
        let sender = kid(1);
        let message = msg_id(1);
        let session_id = session(1);

        cache
            .check_and_record(sender, message, check(session_id, 10_000, 10_000))
            .expect("first message should be fresh");
        let error = cache
            .check_and_record(sender, message, check(session_id, 10_001, 10_001))
            .expect_err("duplicate should be rejected");
        assert!(
            matches!(error, SignalingError::Protocol(message) if message.contains("duplicate message detected"))
        );
    }

    #[test]
    fn expired_entry_is_fresh_after_prune_window_passes() {
        let mut cache = ReplayCache::new(8);
        let sender = kid(2);
        let message = msg_id(2);
        let session_id = session(2);
        let max_message_age_secs = 1;

        let old_check = ReplayCheck { max_message_age_secs, ..check(session_id, 1_000, 1_000) };
        cache.check_and_record(sender, message, old_check).expect("initial insert should succeed");

        let later_check = ReplayCheck { max_message_age_secs, ..check(session(9), 2_100, 2_100) };
        cache
            .check_and_record(sender, msg_id(9), later_check)
            .expect("later insert should trigger prune");
        assert!(
            !cache.entries.contains_key(&ReplayKey { sender_kid: sender, msg_id: message }),
            "expired key should be pruned"
        );

        let replayed_check =
            ReplayCheck { max_message_age_secs, ..check(session_id, 2_101, 2_101) };
        let status = cache
            .check_and_record_status(sender, message, replayed_check)
            .expect("expired entry should be considered fresh");
        assert_eq!(status, ReplayStatus::Fresh);
    }

    #[test]
    fn over_capacity_prunes_oldest_entries() {
        let mut cache = ReplayCache::new(1);
        let sender = kid(3);
        let first = msg_id(3);
        let second = msg_id(4);

        cache.check_and_record(sender, first, check(session(3), 100, 100)).expect("first insert");
        cache.check_and_record(sender, second, check(session(4), 101, 101)).expect("second insert");

        assert_eq!(cache.entries.len(), 1);
        assert!(cache.entries.contains_key(&ReplayKey { sender_kid: sender, msg_id: second }));
        assert!(!cache.entries.contains_key(&ReplayKey { sender_kid: sender, msg_id: first }));
    }

    #[test]
    fn stale_queue_entry_does_not_remove_newer_map_entry() {
        let mut cache = ReplayCache::new(8);
        let key = ReplayKey { sender_kid: kid(4), msg_id: msg_id(4) };
        let old_ts = 100_u64;
        let new_ts = 500_u64;
        cache.order.push_back((key, old_ts));
        cache.order.push_back((key, new_ts));
        cache.entries.insert(key, ReplayEntry { session_id: session(4), timestamp_ms: new_ts });

        cache.prune(790, 300);

        assert!(
            cache.entries.contains_key(&key),
            "stale queue entry must not evict newer replay entry"
        );
        assert_eq!(cache.entries[&key].timestamp_ms, new_ts);
    }

    #[test]
    fn non_monotonic_now_does_not_break_duplicate_detection() {
        let mut cache = ReplayCache::new(4);
        let sender = kid(5);
        let message = msg_id(5);
        let first = check(session(5), 5_000, 5_000);
        let earlier_now = ReplayCheck { now_ms: 4_500, ..check(session(5), 5_000, 5_000) };

        cache.check_and_record(sender, message, first).expect("first insert should succeed");
        let status = cache
            .check_and_record_status(sender, message, earlier_now)
            .expect("duplicate check should still complete");
        assert_eq!(status, ReplayStatus::DuplicateSameSession);
    }
}

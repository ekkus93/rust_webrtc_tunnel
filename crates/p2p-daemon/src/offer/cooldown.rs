//! Probe-failure cooldown for the offer daemon.
//!
//! The offer negotiates lazily, per accepted local client. The existing
//! `enable_auto_reconnect` backoff governs *reconnect / ICE-restart* of an already
//! negotiated session — it does **not** cover a *fresh* client-triggered negotiation that
//! keeps failing the post-DCEP data-plane probe. Without a guard, a retrying local client
//! (e.g. a browser reload loop) against a persistently black-holed data plane would drive
//! a tight `negotiate -> probe-fail -> negotiate -> probe-fail` loop.
//!
//! This cooldown bounds that: after a probe failure to the remote peer, new local clients
//! are refused (their connection dropped, cheaply) until the cooldown elapses, with
//! exponential backoff up to a cap. A non-probe session outcome resets it. The offer
//! daemon serves a single remote peer, so one cooldown instance is "per remote peer".

use std::time::Duration;

use tokio::time::Instant;

/// Tracks the probe-failure backoff window for the offer daemon's remote peer.
#[derive(Debug)]
pub(crate) struct ProbeFailureCooldown {
    /// When the current cooldown window ends, if one is active.
    until: Option<Instant>,
    /// The duration the *next* failure will wait (doubles each consecutive failure).
    next: Duration,
}

impl ProbeFailureCooldown {
    /// Wait applied on the first probe failure.
    const INITIAL: Duration = Duration::from_secs(2);
    /// Cap on the per-failure wait.
    const MAX: Duration = Duration::from_secs(30);

    pub(crate) fn new() -> Self {
        Self { until: None, next: Self::INITIAL }
    }

    /// Remaining cooldown at `now`, or `None` if no cooldown is active.
    pub(crate) fn remaining(&self, now: Instant) -> Option<Duration> {
        self.until.and_then(|until| until.checked_duration_since(now)).filter(|d| !d.is_zero())
    }

    /// Record a probe failure at `now`, opening (or extending) the cooldown window with
    /// exponential backoff. Returns the wait that was applied.
    pub(crate) fn record_failure(&mut self, now: Instant) -> Duration {
        let wait = self.next;
        self.until = Some(now + wait);
        self.next = (self.next * 2).min(Self::MAX);
        wait
    }

    /// Clear the cooldown after a non-probe session outcome.
    pub(crate) fn reset(&mut self) {
        self.until = None;
        self.next = Self::INITIAL;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_cooldown_until_a_failure_is_recorded() {
        let cooldown = ProbeFailureCooldown::new();
        assert_eq!(cooldown.remaining(Instant::now()), None);
    }

    #[test]
    fn failure_opens_a_window_that_expires() {
        let mut cooldown = ProbeFailureCooldown::new();
        let now = Instant::now();
        let wait = cooldown.record_failure(now);
        assert_eq!(wait, ProbeFailureCooldown::INITIAL);
        assert!(cooldown.remaining(now).is_some());
        // After the window the cooldown is no longer active.
        assert_eq!(cooldown.remaining(now + wait), None);
    }

    #[test]
    fn consecutive_failures_back_off_exponentially_to_a_cap() {
        let mut cooldown = ProbeFailureCooldown::new();
        let now = Instant::now();
        assert_eq!(cooldown.record_failure(now), Duration::from_secs(2));
        assert_eq!(cooldown.record_failure(now), Duration::from_secs(4));
        assert_eq!(cooldown.record_failure(now), Duration::from_secs(8));
        assert_eq!(cooldown.record_failure(now), Duration::from_secs(16));
        // Capped at MAX thereafter.
        assert_eq!(cooldown.record_failure(now), ProbeFailureCooldown::MAX);
        assert_eq!(cooldown.record_failure(now), ProbeFailureCooldown::MAX);
    }

    #[test]
    fn reset_clears_window_and_backoff() {
        let mut cooldown = ProbeFailureCooldown::new();
        let now = Instant::now();
        cooldown.record_failure(now);
        cooldown.record_failure(now);
        cooldown.reset();
        assert_eq!(cooldown.remaining(now), None);
        // Backoff restarts from INITIAL after a reset.
        assert_eq!(cooldown.record_failure(now), ProbeFailureCooldown::INITIAL);
    }
}

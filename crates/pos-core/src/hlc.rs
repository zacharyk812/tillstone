//! Hybrid logical clocks (PLAN §6): timestamps that follow wall-clock time
//! when clocks are right, and still order events correctly when they aren't.

use serde::{Deserialize, Serialize};

use crate::ids::DeviceId;

/// A hybrid logical timestamp. Ordered by wall time, then counter, then
/// device, so any two timestamps from different devices compare
/// deterministically on every peer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Hlc {
    /// Milliseconds since the Unix epoch.
    pub wall_ms: u64,
    pub counter: u32,
    pub device: DeviceId,
}

/// How far ahead of our own clock a remote timestamp may be before we treat
/// the remote clock as wrong (PLAN §13: warn on large skew).
pub const MAX_FORWARD_SKEW_MS: u64 = 5 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockWarning {
    /// A peer's timestamp is far ahead of our clock. One of the clocks is wrong.
    RemoteAhead { by_ms: u64 },
}

/// A device's clock state.
#[derive(Debug, Clone)]
pub struct HlcClock {
    device: DeviceId,
    last: Hlc,
}

impl HlcClock {
    pub fn new(device: DeviceId) -> Self {
        Self { device, last: Hlc { wall_ms: 0, counter: 0, device } }
    }

    /// Restores a clock from the last timestamp this device issued.
    pub fn resume(last: Hlc) -> Self {
        Self { device: last.device, last }
    }

    /// A new timestamp for a local event, strictly greater than any seen.
    pub fn tick(&mut self, now_ms: u64) -> Hlc {
        self.last = if now_ms > self.last.wall_ms {
            Hlc { wall_ms: now_ms, counter: 0, device: self.device }
        } else {
            Hlc { wall_ms: self.last.wall_ms, counter: self.last.counter + 1, device: self.device }
        };
        self.last
    }

    /// Merges a timestamp received from a peer, so our next tick sorts after it.
    pub fn observe(&mut self, remote: Hlc, now_ms: u64) -> Option<ClockWarning> {
        let warning = (remote.wall_ms > now_ms + MAX_FORWARD_SKEW_MS)
            .then(|| ClockWarning::RemoteAhead { by_ms: remote.wall_ms - now_ms });
        let wall = now_ms.max(self.last.wall_ms).max(remote.wall_ms);
        let counter = match (wall == self.last.wall_ms, wall == remote.wall_ms) {
            (true, true) => self.last.counter.max(remote.counter) + 1,
            (true, false) => self.last.counter + 1,
            (false, true) => remote.counter + 1,
            (false, false) => 0,
        };
        self.last = Hlc { wall_ms: wall, counter, device: self.device };
        warning
    }

    pub fn last(&self) -> Hlc {
        self.last
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: DeviceId = DeviceId([1; 32]);
    const B: DeviceId = DeviceId([2; 32]);

    #[test]
    fn ticks_are_strictly_increasing_even_if_the_clock_goes_backwards() {
        let mut c = HlcClock::new(A);
        let t1 = c.tick(1_000);
        let t2 = c.tick(1_000);
        let t3 = c.tick(500); // clock jumped back
        let t4 = c.tick(2_000);
        assert!(t1 < t2 && t2 < t3 && t3 < t4);
        assert_eq!(t3.wall_ms, 1_000);
    }

    #[test]
    fn observing_a_peer_orders_later_events_after_it() {
        let mut a = HlcClock::new(A);
        let mut b = HlcClock::new(B);
        let ta = a.tick(10_000);
        // B's clock is 3 seconds behind, but its next event still sorts after A's.
        b.observe(ta, 7_000);
        let tb = b.tick(7_000);
        assert!(tb > ta);
    }

    #[test]
    fn warns_on_large_forward_skew() {
        let mut a = HlcClock::new(A);
        let remote = Hlc { wall_ms: 1_000 + MAX_FORWARD_SKEW_MS + 1, counter: 0, device: B };
        assert!(matches!(a.observe(remote, 1_000), Some(ClockWarning::RemoteAhead { .. })));
        assert_eq!(a.observe(Hlc { wall_ms: 1_500, counter: 0, device: B }, 1_000), None);
    }

    #[test]
    fn ties_break_by_device() {
        let x = Hlc { wall_ms: 5, counter: 1, device: A };
        let y = Hlc { wall_ms: 5, counter: 1, device: B };
        assert!(x < y);
    }
}

//! A tiny fixed-capacity ring buffer used for the TUI's sparkline histories.
//!
//! Pure data structure — no I/O, no platform code — so it is unit-tested
//! inline on every OS.

use std::collections::VecDeque;

/// A bounded FIFO of `u64` samples: pushing past the capacity drops the oldest
/// value, so the buffer always holds the most recent `capacity` samples.
#[derive(Debug, Clone, Default)]
pub struct History {
    data: VecDeque<u64>,
    capacity: usize,
}

impl History {
    /// Create an empty history holding at most `capacity` samples.
    ///
    /// A capacity of `0` yields a buffer that silently discards every push.
    pub fn new(capacity: usize) -> Self {
        History { data: VecDeque::with_capacity(capacity), capacity }
    }

    /// Append a sample, evicting the oldest one when the buffer is full.
    pub fn push(&mut self, value: u64) {
        if self.capacity == 0 {
            return;
        }
        while self.data.len() >= self.capacity {
            self.data.pop_front();
        }
        self.data.push_back(value);
    }

    /// Number of samples currently stored.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// True when no samples have been recorded yet.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Maximum number of samples this buffer keeps.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Iterate the samples oldest-first.
    pub fn iter(&self) -> impl Iterator<Item = &u64> {
        self.data.iter()
    }

    /// The most recently pushed sample, if any.
    pub fn last(&self) -> Option<u64> {
        self.data.back().copied()
    }

    /// The largest sample currently stored.
    pub fn max(&self) -> u64 {
        self.data.iter().copied().max().unwrap_or(0)
    }

    /// Iterate the newest `n` samples, oldest-first.
    ///
    /// Used to fit the history to a sparkline's width: `Sparkline` only draws
    /// the first `area.width` samples, so anything beyond that would silently
    /// drop the *newest* values instead of the oldest.
    pub fn tail(&self, n: usize) -> impl Iterator<Item = &u64> {
        self.data.iter().skip(self.data.len().saturating_sub(n))
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_empty() {
        let h = History::new(4);
        assert!(h.is_empty());
        assert_eq!(h.len(), 0);
        assert_eq!(h.capacity(), 4);
        assert_eq!(h.last(), None);
        assert_eq!(h.max(), 0);
    }

    #[test]
    fn pushes_in_order() {
        let mut h = History::new(4);
        h.push(1);
        h.push(2);
        h.push(3);
        assert_eq!(h.len(), 3);
        assert_eq!(h.iter().copied().collect::<Vec<_>>(), vec![1, 2, 3]);
        assert_eq!(h.last(), Some(3));
    }

    #[test]
    fn evicts_oldest_when_full() {
        let mut h = History::new(3);
        for v in 1..=5 {
            h.push(v);
        }
        assert_eq!(h.len(), 3);
        assert_eq!(h.iter().copied().collect::<Vec<_>>(), vec![3, 4, 5]);
        assert_eq!(h.last(), Some(5));
        assert_eq!(h.max(), 5);
    }

    #[test]
    fn zero_capacity_discards_everything() {
        let mut h = History::new(0);
        h.push(42);
        h.push(43);
        assert!(h.is_empty());
        assert_eq!(h.last(), None);
        assert_eq!(h.tail(10).count(), 0);
    }

    #[test]
    fn tail_returns_newest_samples_oldest_first() {
        let mut h = History::new(8);
        for v in 1..=6 {
            h.push(v);
        }
        assert_eq!(h.tail(3).copied().collect::<Vec<_>>(), vec![4, 5, 6]);
        // Asking for more than we hold returns everything.
        assert_eq!(h.tail(100).copied().collect::<Vec<_>>(), vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(h.tail(0).count(), 0);
    }

    #[test]
    fn max_tracks_current_window_only() {
        let mut h = History::new(2);
        h.push(99);
        h.push(1);
        h.push(2);
        // 99 has been evicted.
        assert_eq!(h.max(), 2);
    }

    #[test]
    fn default_is_zero_capacity() {
        let mut h = History::default();
        h.push(7);
        assert!(h.is_empty());
    }
}

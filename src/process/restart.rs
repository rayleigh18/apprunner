//! Restart policy and crash counter management.

/// Maximum number of retries before giving up.
pub const MAX_RETRIES: u8 = 5;

/// Tracks crash count and determines whether a process should be restarted.
#[derive(Debug)]
pub struct RestartPolicy {
    crash_count: u8,
    max_retries: u8,
}

impl RestartPolicy {
    pub fn new() -> Self {
        Self {
            crash_count: 0,
            max_retries: MAX_RETRIES,
        }
    }

    /// Record a crash. Returns `true` if the process should be retried,
    /// `false` if the maximum retry count has been reached.
    pub fn record_crash(&mut self) -> bool {
        self.crash_count += 1;
        self.crash_count <= self.max_retries
    }

    /// Reset the crash counter (e.g. after a successful manual stop/start).
    pub fn reset(&mut self) {
        self.crash_count = 0;
    }

    /// Current number of crashes recorded.
    pub fn crash_count(&self) -> u8 {
        self.crash_count
    }

    /// Whether the policy has exhausted all retries.
    pub fn is_exhausted(&self) -> bool {
        self.crash_count >= self.max_retries
    }
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crash_counter_increments() {
        let mut policy = RestartPolicy::new();
        assert_eq!(policy.crash_count(), 0);
        policy.record_crash();
        assert_eq!(policy.crash_count(), 1);
        policy.record_crash();
        assert_eq!(policy.crash_count(), 2);
    }

    #[test]
    fn test_reset_clears_counter() {
        let mut policy = RestartPolicy::new();
        policy.record_crash();
        policy.record_crash();
        assert_eq!(policy.crash_count(), 2);
        policy.reset();
        assert_eq!(policy.crash_count(), 0);
    }

    #[test]
    fn test_record_crash_returns_false_at_max() {
        let mut policy = RestartPolicy::new();
        // First MAX_RETRIES crashes should return true
        for _ in 0..MAX_RETRIES {
            assert!(policy.record_crash());
        }
        // The next one should return false (exceeded)
        assert!(!policy.record_crash());
    }

    #[test]
    fn test_is_exhausted() {
        let mut policy = RestartPolicy::new();
        assert!(!policy.is_exhausted());

        for _ in 0..MAX_RETRIES {
            policy.record_crash();
        }
        assert!(policy.is_exhausted());
    }
}

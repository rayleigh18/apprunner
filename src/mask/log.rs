//! Request log ring buffer for API masks.

use std::collections::VecDeque;
use std::time::Instant;

/// Maximum number of log entries retained per mask.
const MAX_LOG_ENTRIES: usize = 50;

/// A single request log entry.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub method: String,
    pub path: String,
    /// HTTP status code from upstream. None if connection failed.
    pub status_code: Option<u16>,
    /// Time to first byte in milliseconds.
    pub latency_ms: u64,
    /// Error message if the request failed (e.g. connection refused).
    /// Never contains header values.
    pub error: Option<String>,
}

/// Fixed-size ring buffer of request log entries.
#[derive(Debug, Clone)]
pub struct RequestLog {
    entries: VecDeque<LogEntry>,
}

impl RequestLog {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::with_capacity(MAX_LOG_ENTRIES),
        }
    }

    /// Add a new log entry, evicting the oldest if at capacity.
    pub fn push(&mut self, entry: LogEntry) {
        if self.entries.len() >= MAX_LOG_ENTRIES {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    /// Get all entries (oldest first).
    pub fn entries(&self) -> &VecDeque<LogEntry> {
        &self.entries
    }

    /// Number of entries currently stored.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the log is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Total number of requests processed (including evicted entries).
    /// For display in the TUI list — tracked separately if needed.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl Default for RequestLog {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to create a timestamp string for log entries.
pub fn now_timestamp() -> String {
    // Use a simple format: YYYY-MM-DD HH:MM:SS
    // We avoid pulling in chrono by using the system time formatting
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();

    // Simple UTC timestamp calculation
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Days since epoch to date (simplified)
    let (year, month, day) = days_to_date(days);

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, month, day, hours, minutes, seconds
    )
}

/// Convert days since Unix epoch to (year, month, day).
fn days_to_date(days: u64) -> (u64, u64, u64) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Measure latency from a start instant (returns ms).
pub fn latency_since(start: Instant) -> u64 {
    start.elapsed().as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_capacity() {
        let mut log = RequestLog::new();

        // Fill beyond capacity
        for i in 0..60 {
            log.push(LogEntry {
                timestamp: format!("2024-01-01 00:00:{:02}", i % 60),
                method: "GET".to_string(),
                path: format!("/path/{}", i),
                status_code: Some(200),
                latency_ms: 100,
                error: None,
            });
        }

        assert_eq!(log.len(), MAX_LOG_ENTRIES);
        // Oldest should be entry 10 (first 10 were evicted)
        assert_eq!(log.entries().front().unwrap().path, "/path/10");
        // Newest should be entry 59
        assert_eq!(log.entries().back().unwrap().path, "/path/59");
    }

    #[test]
    fn test_empty_log() {
        let log = RequestLog::new();
        assert!(log.is_empty());
        assert_eq!(log.len(), 0);
    }

    #[test]
    fn test_push_and_retrieve() {
        let mut log = RequestLog::new();
        log.push(LogEntry {
            timestamp: "2024-01-01 10:00:00".to_string(),
            method: "POST".to_string(),
            path: "/v1/chat/completions".to_string(),
            status_code: Some(200),
            latency_ms: 1200,
            error: None,
        });

        assert_eq!(log.len(), 1);
        let entry = log.entries().front().unwrap();
        assert_eq!(entry.method, "POST");
        assert_eq!(entry.path, "/v1/chat/completions");
        assert_eq!(entry.status_code, Some(200));
        assert_eq!(entry.latency_ms, 1200);
    }

    #[test]
    fn test_error_entry() {
        let mut log = RequestLog::new();
        log.push(LogEntry {
            timestamp: "2024-01-01 10:00:00".to_string(),
            method: "POST".to_string(),
            path: "/v1/chat/completions".to_string(),
            status_code: None,
            latency_ms: 50,
            error: Some("connection refused".to_string()),
        });

        let entry = log.entries().front().unwrap();
        assert!(entry.status_code.is_none());
        assert_eq!(entry.error.as_deref(), Some("connection refused"));
    }

    #[test]
    fn test_clear() {
        let mut log = RequestLog::new();
        log.push(LogEntry {
            timestamp: "2024-01-01 10:00:00".to_string(),
            method: "GET".to_string(),
            path: "/test".to_string(),
            status_code: Some(200),
            latency_ms: 10,
            error: None,
        });

        log.clear();
        assert!(log.is_empty());
    }
}

//! CPU and memory metrics collection via sysinfo.

use sysinfo::{Pid, System};

#[derive(Debug, Clone)]
pub struct ProcessMetrics {
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub memory_display: String,
    pub cpu_display: String,
}

pub struct MetricsCollector {
    system: System,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            system: System::new(),
        }
    }

    /// Refresh system process information. Call this on each tick (every 2s).
    pub fn refresh(&mut self) {
        self.system
            .refresh_processes(sysinfo::ProcessesToUpdate::All);
    }

    /// Get metrics for a specific PID. Returns None if process doesn't exist.
    pub fn get_metrics(&self, pid: u32) -> Option<ProcessMetrics> {
        let sysinfo_pid = Pid::from(pid as usize);
        let process = self.system.process(sysinfo_pid)?;

        let cpu_percent = process.cpu_usage();
        let memory_bytes = process.memory();

        Some(ProcessMetrics {
            cpu_percent,
            memory_bytes,
            memory_display: Self::format_memory(memory_bytes),
            cpu_display: Self::format_cpu(cpu_percent),
        })
    }

    /// Format bytes into human-readable string.
    pub fn format_memory(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = 1024 * 1024;
        const GB: u64 = 1024 * 1024 * 1024;

        if bytes < KB {
            format!("{}B", bytes)
        } else if bytes < MB {
            let val = bytes as f64 / KB as f64;
            if val.fract() == 0.0 {
                format!("{}K", val as u64)
            } else {
                format!("{:.1}K", val)
            }
        } else if bytes < GB {
            let val = bytes as f64 / MB as f64;
            if val.fract() == 0.0 {
                format!("{}M", val as u64)
            } else {
                format!("{:.1}M", val)
            }
        } else {
            let val = bytes as f64 / GB as f64;
            format!("{:.1}G", val)
        }
    }

    /// Format CPU percentage into display string.
    pub fn format_cpu(percent: f32) -> String {
        format!("{:.1}%", percent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_memory_bytes() {
        assert_eq!(MetricsCollector::format_memory(0), "0B");
        assert_eq!(MetricsCollector::format_memory(512), "512B");
    }

    #[test]
    fn test_format_memory_kilobytes() {
        assert_eq!(MetricsCollector::format_memory(1024), "1K");
        assert_eq!(MetricsCollector::format_memory(1536), "1.5K");
    }

    #[test]
    fn test_format_memory_megabytes() {
        assert_eq!(MetricsCollector::format_memory(1_048_576), "1M");
        assert_eq!(MetricsCollector::format_memory(88_080_384), "84M");
    }

    #[test]
    fn test_format_memory_gigabytes() {
        assert_eq!(MetricsCollector::format_memory(1_288_490_188), "1.2G");
        assert_eq!(MetricsCollector::format_memory(2_147_483_648), "2.0G");
    }

    #[test]
    fn test_format_cpu() {
        assert_eq!(MetricsCollector::format_cpu(0.0), "0.0%");
        assert_eq!(MetricsCollector::format_cpu(3.14159), "3.1%");
        assert_eq!(MetricsCollector::format_cpu(100.0), "100.0%");
    }

    #[test]
    fn test_get_metrics_nonexistent_pid() {
        let collector = MetricsCollector::new();
        // PID 0 or a very high PID should not exist as a user process
        assert!(collector.get_metrics(u32::MAX).is_none());
    }

    #[test]
    fn test_get_metrics_current_process() {
        let mut collector = MetricsCollector::new();
        collector.refresh();
        let pid = std::process::id();
        let metrics = collector.get_metrics(pid);
        assert!(
            metrics.is_some(),
            "Expected Some for current process PID {pid}"
        );
    }

    #[test]
    fn test_refresh_does_not_panic() {
        let mut collector = MetricsCollector::new();
        collector.refresh();
        collector.refresh();
    }
}

//! Runtime alert logic — monitors apps for exceeding max runtime.

use std::time::Duration;

use crate::db::models::AppConfig;
use crate::process::ProcessState;

#[derive(Debug, Clone)]
pub struct RuntimeAlert {
    pub app_id: i64,
    pub app_name: String,
    pub exceeded_by: Duration,
}

/// Check all running processes for runtime exceedance.
///
/// For each app:
/// - If it has `max_runtime_secs` set, use that as the threshold
/// - Otherwise, use `global_max_secs` (default: 18000 = 5 hours)
/// - If the process has been running longer than the threshold, create an alert
///
/// Returns a list of alerts for apps that have exceeded their runtime.
pub fn check_runtime_alerts(
    apps: &[AppConfig],
    process_states: &[(i64, &ProcessState)],
    global_max_secs: u64,
) -> Vec<RuntimeAlert> {
    let mut alerts = Vec::new();

    for app in apps {
        // Find matching process state
        let state = process_states.iter().find(|(id, _)| *id == app.id);

        if let Some((_, ProcessState::Running { started_at, .. })) = state {
            let threshold_secs = app.max_runtime_secs.unwrap_or(global_max_secs as i64) as u64;
            let threshold = Duration::from_secs(threshold_secs);
            let elapsed = started_at.elapsed();

            if elapsed > threshold {
                alerts.push(RuntimeAlert {
                    app_id: app.id,
                    app_name: app.name.clone(),
                    exceeded_by: elapsed - threshold,
                });
            }
        }
    }

    alerts
}

/// Format a duration into a human-readable string.
///
/// - < 60s: "{s}s"
/// - < 3600s: "{m}m"
/// - >= 3600s: "{h}h {m}m" (omit minutes if 0)
pub fn format_duration(d: Duration) -> String {
    let total_secs = d.as_secs();

    if total_secs < 60 {
        format!("{}s", total_secs)
    } else if total_secs < 3600 {
        let minutes = total_secs / 60;
        format!("{}m", minutes)
    } else {
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        if minutes == 0 {
            format!("{}h", hours)
        } else {
            format!("{}h {}m", hours, minutes)
        }
    }
}

/// Format an alert into a display string for the status bar.
pub fn format_alert(alert: &RuntimeAlert) -> String {
    format!(
        "\u{23f1} {}: running {} over limit",
        alert.app_name,
        format_duration(alert.exceeded_by)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn make_running_state(elapsed_secs: u64) -> ProcessState {
        ProcessState::Running {
            pid: 1234,
            started_at: Instant::now() - Duration::from_secs(elapsed_secs),
        }
    }

    fn make_app(id: i64, name: &str, max_runtime_secs: Option<i64>) -> AppConfig {
        AppConfig {
            id,
            name: name.to_string(),
            working_dir: "/tmp".to_string(),
            command: "echo hi".to_string(),
            env_vars: "{}".to_string(),
            auto_start: false,
            max_runtime_secs,
            interval_seconds: None,
            template_vars: vec![],
            created_at: "2024-01-01".to_string(),
        }
    }

    #[test]
    fn test_no_running_apps_returns_empty() {
        let apps = vec![make_app(1, "app1", None)];
        let states: Vec<(i64, &ProcessState)> = vec![];
        let alerts = check_runtime_alerts(&apps, &states, 18000);
        assert!(alerts.is_empty());
    }

    #[test]
    fn test_running_app_under_threshold_returns_empty() {
        let apps = vec![make_app(1, "app1", Some(300))];
        let state = make_running_state(100); // 100s running, threshold 300s
        let states: Vec<(i64, &ProcessState)> = vec![(1, &state)];
        let alerts = check_runtime_alerts(&apps, &states, 18000);
        assert!(alerts.is_empty());
    }

    #[test]
    fn test_running_app_over_global_threshold_returns_alert() {
        let apps = vec![make_app(1, "app1", None)]; // No per-app limit
        let state = make_running_state(20000); // 20000s running, global 18000s
        let states: Vec<(i64, &ProcessState)> = vec![(1, &state)];
        let alerts = check_runtime_alerts(&apps, &states, 18000);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].app_id, 1);
        assert_eq!(alerts[0].app_name, "app1");
        // exceeded_by should be approximately 2000s
        assert!(alerts[0].exceeded_by.as_secs() >= 1999);
    }

    #[test]
    fn test_per_app_threshold_takes_precedence_over_global() {
        let apps = vec![make_app(1, "app1", Some(60))]; // Per-app: 60s
        let state = make_running_state(100); // 100s running
        let states: Vec<(i64, &ProcessState)> = vec![(1, &state)];

        // Global is 18000 (would not trigger), but per-app is 60 (should trigger)
        let alerts = check_runtime_alerts(&apps, &states, 18000);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].app_id, 1);
        // exceeded_by should be approximately 40s
        assert!(alerts[0].exceeded_by.as_secs() >= 39);
    }

    #[test]
    fn test_stopped_app_produces_no_alert() {
        let apps = vec![make_app(1, "app1", Some(60))];
        let state = ProcessState::Stopped;
        let states: Vec<(i64, &ProcessState)> = vec![(1, &state)];
        let alerts = check_runtime_alerts(&apps, &states, 18000);
        assert!(alerts.is_empty());
    }

    #[test]
    fn test_crashed_app_produces_no_alert() {
        let apps = vec![make_app(1, "app1", Some(60))];
        let state = ProcessState::Crashed {
            retries: 3,
            last_error: "fail".to_string(),
        };
        let states: Vec<(i64, &ProcessState)> = vec![(1, &state)];
        let alerts = check_runtime_alerts(&apps, &states, 18000);
        assert!(alerts.is_empty());
    }

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(Duration::from_secs(0)), "0s");
        assert_eq!(format_duration(Duration::from_secs(59)), "59s");
    }

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(format_duration(Duration::from_secs(60)), "1m");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m");
        assert_eq!(format_duration(Duration::from_secs(3599)), "59m");
    }

    #[test]
    fn test_format_duration_hours() {
        assert_eq!(format_duration(Duration::from_secs(3600)), "1h");
        assert_eq!(format_duration(Duration::from_secs(3661)), "1h 1m");
        assert_eq!(format_duration(Duration::from_secs(7200)), "2h");
        assert_eq!(format_duration(Duration::from_secs(7260)), "2h 1m");
    }

    #[test]
    fn test_format_alert_produces_expected_string() {
        let alert = RuntimeAlert {
            app_id: 1,
            app_name: "my-server".to_string(),
            exceeded_by: Duration::from_secs(300),
        };
        let result = format_alert(&alert);
        assert_eq!(result, "\u{23f1} my-server: running 5m over limit");
    }
}

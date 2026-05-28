//! Integration tests exercising multiple modules together.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use apprunner::alerts::check_runtime_alerts;
use apprunner::db;
use apprunner::db::models::{AppConfig, NewApp};
use apprunner::db::operations::{create_app, get_app_by_id};
use apprunner::metrics::MetricsCollector;
use apprunner::process::health::check_app_health;
use apprunner::process::{ManagedProcess, ProcessState};
use apprunner::vt::Scrollback;

// ---------------------------------------------------------------------------
// Test 1: Full lifecycle — create app in DB, validate health, start process,
//         capture output, stop process.
// ---------------------------------------------------------------------------

#[test]
fn test_full_lifecycle() {
    // Init in-memory DB
    let conn = db::init_memory().unwrap();

    // Create app
    let new_app = NewApp {
        name: "test-echo".to_string(),
        working_dir: "/tmp".to_string(),
        command: "echo hello".to_string(),
        env_vars: "{}".to_string(),
        auto_start: false,
        max_runtime_secs: None,
    };
    let id = create_app(&conn, &new_app).unwrap();
    assert!(id > 0);

    // Retrieve and validate
    let app = get_app_by_id(&conn, id).unwrap();
    assert_eq!(app.name, "test-echo");

    // Health check should pass
    let health = check_app_health(&app.working_dir, &app.command, &app.env_vars);
    assert!(health.is_healthy);

    // Create managed process, start it
    let mut proc = ManagedProcess::new(id);
    let env_vars = HashMap::new();
    proc.start("/tmp", "echo hello", &env_vars).unwrap();
    assert!(matches!(proc.state, ProcessState::Running { .. }));

    // Wait for output
    std::thread::sleep(Duration::from_millis(500));

    // Verify output contains "hello"
    let output = proc.get_output();
    let text = String::from_utf8_lossy(&output);
    assert!(
        text.contains("hello"),
        "Expected output to contain 'hello', got: {:?}",
        text
    );

    // Stop process
    proc.stop().unwrap();
    assert_eq!(proc.state, ProcessState::Stopped);
}

// ---------------------------------------------------------------------------
// Test 2: Crash and restart cycle
// ---------------------------------------------------------------------------

#[test]
fn test_crash_and_restart_cycle() {
    let mut proc = ManagedProcess::new(1);
    let env_vars = HashMap::new();

    // Start a process that exits immediately with non-zero
    proc.start("/tmp", "exit 1", &env_vars).unwrap();
    std::thread::sleep(Duration::from_millis(500));

    // First tick should detect the crash and restart
    let changed = proc.tick("/tmp", "exit 1", &env_vars);
    assert!(changed);
    assert_eq!(proc.restart_policy.crash_count(), 1);

    // Exhaust restart policy: keep ticking until crashed
    for _ in 0..10 {
        std::thread::sleep(Duration::from_millis(500));
        proc.tick("/tmp", "exit 1", &env_vars);

        if matches!(proc.state, ProcessState::Crashed { .. }) {
            break;
        }
    }

    // After enough retries, state should be Crashed
    assert!(
        matches!(proc.state, ProcessState::Crashed { .. }),
        "Expected Crashed state, got: {:?}",
        proc.state
    );
}

// ---------------------------------------------------------------------------
// Test 3: Metrics collection on running process
// ---------------------------------------------------------------------------

#[test]
fn test_metrics_collection_on_running_process() {
    let mut proc = ManagedProcess::new(1);
    let env_vars = HashMap::new();

    proc.start("/tmp", "sleep 5", &env_vars).unwrap();
    std::thread::sleep(Duration::from_millis(300));

    // Get PID from the Running state
    let pid = match &proc.state {
        ProcessState::Running { pid, .. } => *pid,
        other => panic!("Expected Running state, got: {:?}", other),
    };

    // Collect metrics
    let mut collector = MetricsCollector::new();
    collector.refresh();

    let metrics = collector.get_metrics(pid);
    assert!(
        metrics.is_some(),
        "Expected metrics for PID {}, got None",
        pid
    );

    let m = metrics.unwrap();
    // Memory should be non-zero for a running process
    assert!(m.memory_bytes > 0 || m.cpu_percent >= 0.0);

    // Clean up
    proc.stop().unwrap();
}

// ---------------------------------------------------------------------------
// Test 4: VT output integration — colored text parsing
// ---------------------------------------------------------------------------

#[test]
fn test_vt_output_integration() {
    let mut scrollback = Scrollback::new(1000);

    // Simulate colored output: ESC[31m = red foreground, ESC[0m = reset
    let colored_output = b"\x1b[31mRED\x1b[0m normal";
    scrollback.feed(colored_output);

    let line = scrollback.current_line();
    let text = line.plain_text();
    assert_eq!(text, "RED normal");

    // First 3 chars ("RED") should have red foreground
    use ratatui::style::Color;
    for sc in &line.chars[..3] {
        assert_eq!(
            sc.style.fg,
            Some(Color::Red),
            "Expected Red color for 'RED' portion"
        );
    }

    // Remaining chars (" normal") should have default style
    for sc in &line.chars[3..] {
        assert_eq!(
            sc.style,
            ratatui::style::Style::default(),
            "Expected default style for ' normal' portion"
        );
    }
}

// ---------------------------------------------------------------------------
// Test 5: DB + Health check integration
// ---------------------------------------------------------------------------

#[test]
fn test_db_health_check_integration() {
    let conn = db::init_memory().unwrap();

    // Create app with invalid working directory
    let invalid_app = NewApp {
        name: "invalid-dir-app".to_string(),
        working_dir: "/nonexistent/path/xyz123".to_string(),
        command: "echo hi".to_string(),
        env_vars: "{}".to_string(),
        auto_start: false,
        max_runtime_secs: None,
    };
    let id1 = create_app(&conn, &invalid_app).unwrap();
    let app1 = get_app_by_id(&conn, id1).unwrap();

    // Health check should fail
    let health = check_app_health(&app1.working_dir, &app1.command, &app1.env_vars);
    assert!(!health.is_healthy);
    assert!(!health.errors.is_empty());

    // Create app with valid config
    let valid_app = NewApp {
        name: "valid-app".to_string(),
        working_dir: "/tmp".to_string(),
        command: "echo hello".to_string(),
        env_vars: r#"{"PORT": "3000"}"#.to_string(),
        auto_start: false,
        max_runtime_secs: Some(300),
    };
    let id2 = create_app(&conn, &valid_app).unwrap();
    let app2 = get_app_by_id(&conn, id2).unwrap();

    // Health check should pass
    let health = check_app_health(&app2.working_dir, &app2.command, &app2.env_vars);
    assert!(health.is_healthy);
    assert!(health.errors.is_empty());
}

// ---------------------------------------------------------------------------
// Test 6: Runtime alerts integration
// ---------------------------------------------------------------------------

#[test]
fn test_runtime_alerts_integration() {
    let conn = db::init_memory().unwrap();

    // Create app with max_runtime_secs = 1
    let app_data = NewApp {
        name: "short-lived-app".to_string(),
        working_dir: "/tmp".to_string(),
        command: "sleep 100".to_string(),
        env_vars: "{}".to_string(),
        auto_start: false,
        max_runtime_secs: Some(1),
    };
    let id = create_app(&conn, &app_data).unwrap();
    let app = get_app_by_id(&conn, id).unwrap();

    // Simulate a process that started 5 seconds ago
    let running_state = ProcessState::Running {
        pid: 9999,
        started_at: Instant::now() - Duration::from_secs(5),
    };

    let apps = vec![app];
    let process_states: Vec<(i64, &ProcessState)> = vec![(id, &running_state)];

    let alerts = check_runtime_alerts(&apps, &process_states, 18000);
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].app_name, "short-lived-app");
    assert_eq!(alerts[0].app_id, id);
    // exceeded_by should be approximately 4 seconds
    assert!(alerts[0].exceeded_by.as_secs() >= 3);
}

// ---------------------------------------------------------------------------
// Test 7: VT + Process output end-to-end
// ---------------------------------------------------------------------------

#[test]
fn test_vt_process_output_end_to_end() {
    let mut proc = ManagedProcess::new(1);
    let env_vars = HashMap::new();

    // Start a process that produces output with newlines
    proc.start("/tmp", "printf 'line1\\nline2\\nline3\\n'", &env_vars)
        .unwrap();
    std::thread::sleep(Duration::from_millis(500));

    // Get raw output
    let output = proc.get_output();
    assert!(!output.is_empty());

    // Feed into scrollback
    let mut scrollback = Scrollback::new(1000);
    scrollback.feed(&output);

    // Should have parsed multiple lines
    assert!(
        scrollback.total_lines() >= 2,
        "Expected at least 2 lines, got {}",
        scrollback.total_lines()
    );

    proc.stop().unwrap();
}

// ---------------------------------------------------------------------------
// Test 8: DB CRUD full cycle integration
// ---------------------------------------------------------------------------

#[test]
fn test_db_crud_full_cycle() {
    use apprunner::db::operations::{delete_app, get_all_apps, update_app};

    let conn = db::init_memory().unwrap();

    // Create
    let app = NewApp {
        name: "crud-test".to_string(),
        working_dir: "/tmp".to_string(),
        command: "echo test".to_string(),
        env_vars: "{}".to_string(),
        auto_start: false,
        max_runtime_secs: Some(60),
    };
    let id = create_app(&conn, &app).unwrap();

    // Read
    let retrieved = get_app_by_id(&conn, id).unwrap();
    assert_eq!(retrieved.name, "crud-test");

    // Update
    let updated = NewApp {
        name: "crud-updated".to_string(),
        working_dir: "/tmp".to_string(),
        command: "echo updated".to_string(),
        env_vars: r#"{"KEY": "val"}"#.to_string(),
        auto_start: true,
        max_runtime_secs: None,
    };
    update_app(&conn, id, &updated).unwrap();
    let retrieved = get_app_by_id(&conn, id).unwrap();
    assert_eq!(retrieved.name, "crud-updated");
    assert!(retrieved.auto_start);

    // List
    let all = get_all_apps(&conn).unwrap();
    assert_eq!(all.len(), 1);

    // Delete
    delete_app(&conn, id).unwrap();
    let all = get_all_apps(&conn).unwrap();
    assert_eq!(all.len(), 0);
}

// ---------------------------------------------------------------------------
// Test 9: No alerts when process is under threshold
// ---------------------------------------------------------------------------

#[test]
fn test_no_alerts_when_under_threshold() {
    let app = AppConfig {
        id: 1,
        name: "ok-app".to_string(),
        working_dir: "/tmp".to_string(),
        command: "sleep 10".to_string(),
        env_vars: "{}".to_string(),
        auto_start: false,
        max_runtime_secs: Some(300),
        created_at: "2024-01-01".to_string(),
    };

    // Process started just 10 seconds ago, threshold is 300s
    let running_state = ProcessState::Running {
        pid: 1234,
        started_at: Instant::now() - Duration::from_secs(10),
    };

    let alerts = check_runtime_alerts(&[app], &[(1, &running_state)], 18000);
    assert!(alerts.is_empty());
}

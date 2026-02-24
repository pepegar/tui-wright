use std::thread;
use std::time::Duration;

use tui_wright::client;
use tui_wright::protocol::{Request, Response};
use tui_wright::server;

fn spawn_bash_session() -> String {
    let session_id = server::generate_session_id();
    let id = session_id.clone();
    let args: Vec<String> = vec![];
    let cwd = std::env::current_dir().unwrap();
    thread::spawn(move || {
        server::run_daemon("bash", &args, 80, 24, &id, &cwd).ok();
    });
    thread::sleep(Duration::from_millis(500));
    session_id
}

fn cleanup(session_id: &str) {
    let _ = client::send_request(session_id, &Request::Kill);
    thread::sleep(Duration::from_millis(100));
}

#[test]
fn test_spawn_and_screen() {
    let session = spawn_bash_session();
    let resp = client::send_request(&session, &Request::Screen { json: false }).unwrap();
    match resp {
        Response::Text { text } => {
            assert!(text.contains("$") || text.contains("#") || text.contains("bash"));
        }
        other => panic!("Expected Text response, got: {:?}", other),
    }
    cleanup(&session);
}

#[test]
fn test_type_and_read() {
    let session = spawn_bash_session();

    client::send_request(&session, &Request::Type { text: "echo integration_test_marker".into() }).unwrap();
    client::send_request(&session, &Request::Key { name: "enter".into() }).unwrap();
    thread::sleep(Duration::from_millis(300));

    let resp = client::send_request(&session, &Request::Screen { json: false }).unwrap();
    match resp {
        Response::Text { text } => {
            assert!(text.contains("integration_test_marker"), "Screen should contain the echoed text: {}", text);
        }
        other => panic!("Expected Text response, got: {:?}", other),
    }
    cleanup(&session);
}

#[test]
fn test_cursor_position() {
    let session = spawn_bash_session();

    let resp = client::send_request(&session, &Request::Cursor).unwrap();
    match resp {
        Response::Cursor { row, col } => {
            assert!(row < 24);
            assert!(col < 80);
        }
        other => panic!("Expected Cursor response, got: {:?}", other),
    }
    cleanup(&session);
}

#[test]
fn test_json_screen() {
    let session = spawn_bash_session();

    client::send_request(&session, &Request::Type { text: "echo json_test".into() }).unwrap();
    client::send_request(&session, &Request::Key { name: "enter".into() }).unwrap();
    thread::sleep(Duration::from_millis(300));

    let resp = client::send_request(&session, &Request::Screen { json: true }).unwrap();
    match resp {
        Response::Screen { snapshot } => {
            assert_eq!(snapshot.rows, 24);
            assert_eq!(snapshot.cols, 80);
            assert_eq!(snapshot.cells.len(), 24);
            assert_eq!(snapshot.cells[0].len(), 80);
        }
        other => panic!("Expected Screen response, got: {:?}", other),
    }
    cleanup(&session);
}

#[test]
fn test_resize() {
    let session = spawn_bash_session();

    let resp = client::send_request(&session, &Request::Resize { cols: 120, rows: 40 }).unwrap();
    assert!(matches!(resp, Response::Ok));

    thread::sleep(Duration::from_millis(200));

    let resp = client::send_request(&session, &Request::Screen { json: true }).unwrap();
    match resp {
        Response::Screen { snapshot } => {
            assert_eq!(snapshot.rows, 40);
            assert_eq!(snapshot.cols, 120);
        }
        other => panic!("Expected Screen response, got: {:?}", other),
    }
    cleanup(&session);
}

#[test]
fn test_key_arrow() {
    let session = spawn_bash_session();
    let resp = client::send_request(&session, &Request::Key { name: "up".into() }).unwrap();
    assert!(matches!(resp, Response::Ok));
    cleanup(&session);
}

#[test]
fn test_kill() {
    let session = spawn_bash_session();

    let resp = client::send_request(&session, &Request::Kill).unwrap();
    assert!(matches!(resp, Response::Ok));

    thread::sleep(Duration::from_millis(200));
    let result = client::send_request(&session, &Request::Screen { json: false });
    assert!(result.is_err());
}

#[test]
fn test_session_not_found() {
    let result = client::send_request("nonexistent", &Request::Cursor);
    assert!(result.is_err());
}

#[test]
fn test_list_sessions() {
    let session = spawn_bash_session();
    let sessions = client::list_sessions();
    assert!(sessions.contains(&session), "Session {} should be in list {:?}", session, sessions);
    cleanup(&session);
}

#[test]
fn test_trace_start_stop() {
    let session = spawn_bash_session();
    let cast_file = std::env::temp_dir().join(format!("test-trace-{}.cast", &session));

    let resp = client::send_request(&session, &Request::TraceStart {
        output: Some(cast_file.to_string_lossy().to_string()),
    }).unwrap();
    assert!(matches!(resp, Response::Ok));

    client::send_request(&session, &Request::Type { text: "echo trace_test".into() }).unwrap();
    client::send_request(&session, &Request::Key { name: "enter".into() }).unwrap();
    thread::sleep(Duration::from_millis(300));

    let resp = client::send_request(&session, &Request::TraceMarker {
        label: "after-echo".to_string(),
    }).unwrap();
    assert!(matches!(resp, Response::Ok));

    let resp = client::send_request(&session, &Request::TraceStop).unwrap();
    assert!(matches!(resp, Response::Ok));

    let content = std::fs::read_to_string(&cast_file).unwrap();
    let lines: Vec<&str> = content.trim().lines().collect();
    assert!(lines.len() >= 2, "Should have header + at least one event");

    let header: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(header["version"], 2);
    assert_eq!(header["width"], 80);
    assert_eq!(header["height"], 24);

    let has_output = lines[1..].iter().any(|line| {
        let val: serde_json::Value = serde_json::from_str(line).unwrap();
        val[1] == "o"
    });
    assert!(has_output, "Should contain at least one output event");

    let has_marker = lines[1..].iter().any(|line| {
        let val: serde_json::Value = serde_json::from_str(line).unwrap();
        val[1] == "m" && val[2] == "after-echo"
    });
    assert!(has_marker, "Should contain the custom marker event");

    let has_input = lines[1..].iter().any(|line| {
        let val: serde_json::Value = serde_json::from_str(line).unwrap();
        val[1] == "i"
    });
    assert!(has_input, "Should contain input events");

    let _ = std::fs::remove_file(&cast_file);
    cleanup(&session);
}

#[test]
fn test_snapshot_diff_identical() {
    let session = spawn_bash_session();

    client::send_request(&session, &Request::Type { text: "echo snapshot_test".into() }).unwrap();
    client::send_request(&session, &Request::Key { name: "enter".into() }).unwrap();
    thread::sleep(Duration::from_millis(300));

    let resp = client::send_request(&session, &Request::Screen { json: true }).unwrap();
    let baseline = match resp {
        Response::Screen { snapshot } => snapshot,
        other => panic!("Expected Screen response, got: {:?}", other),
    };

    let diff_resp = client::send_request(&session, &Request::SnapshotDiff {
        baseline: baseline.clone(),
    }).unwrap();
    match diff_resp {
        Response::Diff { diff } => {
            assert!(diff.identical, "Immediate diff should be identical");
            assert_eq!(diff.changed_cells.len(), 0);
        }
        other => panic!("Expected Diff response, got: {:?}", other),
    }

    cleanup(&session);
}

#[test]
fn test_snapshot_diff_changed() {
    let session = spawn_bash_session();

    let resp = client::send_request(&session, &Request::Screen { json: true }).unwrap();
    let baseline = match resp {
        Response::Screen { snapshot } => snapshot,
        other => panic!("Expected Screen response, got: {:?}", other),
    };

    client::send_request(&session, &Request::Type { text: "echo changed".into() }).unwrap();
    client::send_request(&session, &Request::Key { name: "enter".into() }).unwrap();
    thread::sleep(Duration::from_millis(300));

    let diff_resp = client::send_request(&session, &Request::SnapshotDiff {
        baseline,
    }).unwrap();
    match diff_resp {
        Response::Diff { diff } => {
            assert!(!diff.identical, "Diff should detect changes");
            assert!(diff.changed_cells.len() > 0, "Should have changed cells");
        }
        other => panic!("Expected Diff response, got: {:?}", other),
    }

    cleanup(&session);
}

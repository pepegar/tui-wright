use std::thread;
use std::time::Duration;

use tui_wright::client;
use tui_wright::protocol::{Request, Response};
use tui_wright::server;

fn spawn_bash_session() -> String {
    let session_id = server::generate_session_id();
    let id = session_id.clone();
    let args: Vec<String> = vec![];
    thread::spawn(move || {
        server::run_daemon("bash", &args, 80, 24, &id).ok();
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

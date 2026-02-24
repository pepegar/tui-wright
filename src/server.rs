use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;

use crate::error::Result;
use crate::protocol::{Request, Response};
use crate::session::Session;

pub fn socket_path(session_id: &str) -> PathBuf {
    let tmp = std::env::temp_dir();
    tmp.join(format!("tui-wright-{}.sock", session_id))
}

pub fn generate_session_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    format!("{:06x}", rng.gen::<u32>() & 0xFFFFFF)
}

pub fn run_daemon(command: &str, args: &[String], cols: u16, rows: u16, session_id: &str, cwd: &std::path::Path) -> Result<()> {
    let sock = socket_path(session_id);
    if sock.exists() {
        std::fs::remove_file(&sock)?;
    }

    let listener = UnixListener::bind(&sock)?;
    let mut session = Session::spawn(command, args, cols, rows, cwd)?;

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };

        let mut reader = BufReader::new(&stream);
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() {
            continue;
        }

        let request: Request = match serde_json::from_str(line.trim()) {
            Ok(r) => r,
            Err(e) => {
                let resp = Response::Error { message: format!("Invalid request: {}", e) };
                let _ = write_response(&stream, &resp);
                continue;
            }
        };

        let response = handle_request(&mut session, request);
        let _ = write_response(&stream, &response);

        if matches!(response, Response::Ok) {
            if let Ok(serde_json::Value::String(ref t)) = serde_json::from_str::<serde_json::Value>(line.trim()) {
                let _ = t;
            }
        }

        if line.trim().contains("\"Kill\"") || line.trim().contains("\"type\":\"Kill\"") {
            let _ = std::fs::remove_file(&sock);
            break;
        }
    }

    Ok(())
}

fn handle_request(session: &mut Session, request: Request) -> Response {
    match request {
        Request::Screen { json } => {
            if json {
                Response::Screen { snapshot: session.screen_snapshot() }
            } else {
                Response::Text { text: session.screen_text() }
            }
        }
        Request::Type { text } => match session.type_text(&text) {
            Ok(()) => Response::Ok,
            Err(e) => Response::Error { message: e.to_string() },
        },
        Request::Key { name } => match session.send_key_by_name(&name) {
            Ok(()) => Response::Ok,
            Err(e) => Response::Error { message: e.to_string() },
        },
        Request::Mouse { action, col, row } => match session.send_mouse(&action, col, row) {
            Ok(()) => Response::Ok,
            Err(e) => Response::Error { message: e.to_string() },
        },
        Request::Resize { cols, rows } => match session.resize(cols, rows) {
            Ok(()) => Response::Ok,
            Err(e) => Response::Error { message: e.to_string() },
        },
        Request::Cursor => {
            let (row, col) = session.cursor_position();
            Response::Cursor { row, col }
        }
        Request::Kill => match session.kill() {
            Ok(()) => Response::Ok,
            Err(e) => Response::Error { message: e.to_string() },
        },
    }
}

fn write_response(mut stream: &std::os::unix::net::UnixStream, response: &Response) -> Result<()> {
    let json = serde_json::to_string(response)?;
    stream.write_all(json.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;
    Ok(())
}

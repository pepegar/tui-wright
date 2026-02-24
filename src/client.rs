use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

use crate::error::{Error, Result};
use crate::protocol::{Request, Response};
use crate::server::socket_path;

pub fn send_request(session_id: &str, request: &Request) -> Result<Response> {
    let sock = socket_path(session_id);
    if !sock.exists() {
        return Err(Error::SessionNotFound(session_id.to_string()));
    }

    let mut stream = UnixStream::connect(&sock)?;
    let json = serde_json::to_string(request)?;
    stream.write_all(json.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;

    let response: Response = serde_json::from_str(line.trim())?;
    Ok(response)
}

pub fn print_response(response: &Response) {
    match response {
        Response::Ok => {}
        Response::Text { text } => println!("{}", text),
        Response::Screen { snapshot } => {
            println!("{}", serde_json::to_string_pretty(snapshot).unwrap());
        }
        Response::Cursor { row, col } => println!("row: {}, col: {}", row, col),
        Response::Error { message } => {
            eprintln!("Error: {}", message);
            std::process::exit(1);
        }
    }
}

pub fn list_sessions() -> Vec<String> {
    let tmp = std::env::temp_dir();
    let mut sessions = Vec::new();
    if let Ok(entries) = std::fs::read_dir(tmp) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("tui-wright-") && name.ends_with(".sock") {
                let id = name
                    .strip_prefix("tui-wright-")
                    .unwrap()
                    .strip_suffix(".sock")
                    .unwrap()
                    .to_string();
                sessions.push(id);
            }
        }
    }
    sessions
}

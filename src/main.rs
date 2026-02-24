use clap::{Parser, Subcommand};

use tui_wright::client;
use tui_wright::protocol::{Request, Response};
use tui_wright::server;

#[derive(Parser)]
#[command(name = "tui-wright", about = "Playwright for Terminal UIs")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Spawn a new TUI session in a background daemon
    Spawn {
        /// Command to run
        command: String,
        /// Arguments for the command
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
        /// Terminal columns
        #[arg(long, default_value = "80")]
        cols: u16,
        /// Terminal rows
        #[arg(long, default_value = "24")]
        rows: u16,
    },
    /// Get the current screen contents
    Screen {
        /// Session ID
        session: String,
        /// Output as JSON with cell-level attributes
        #[arg(long)]
        json: bool,
    },
    /// Type text into the session
    Type {
        /// Session ID
        session: String,
        /// Text to type
        text: String,
    },
    /// Send a special key
    Key {
        /// Session ID
        session: String,
        /// Key name (enter, tab, ctrl+c, up, f5, etc.)
        name: String,
    },
    /// Send a mouse event
    Mouse {
        /// Session ID
        session: String,
        /// Mouse action (press, release, move, scrollup, scrolldown)
        action: String,
        /// Column (0-indexed)
        col: u16,
        /// Row (0-indexed)
        row: u16,
    },
    /// Resize the terminal viewport
    Resize {
        /// Session ID
        session: String,
        /// New column count
        cols: u16,
        /// New row count
        rows: u16,
    },
    /// Get the cursor position
    Cursor {
        /// Session ID
        session: String,
    },
    /// Kill a session
    Kill {
        /// Session ID
        session: String,
    },
    /// List active sessions
    List,
    /// Wait until text appears on screen (or timeout)
    WaitFor {
        /// Session ID
        session: String,
        /// Text to wait for
        text: String,
        /// Timeout in milliseconds
        #[arg(long, default_value = "5000")]
        timeout: u64,
    },
    /// Assert that text is currently visible on screen
    Assert {
        /// Session ID
        session: String,
        /// Text to search for
        text: String,
    },
    /// Spawn a session and run a command (spawn + type + enter)
    Run {
        /// Command to run
        command: String,
        /// Terminal columns
        #[arg(long, default_value = "80")]
        cols: u16,
        /// Terminal rows
        #[arg(long, default_value = "24")]
        rows: u16,
    },
    /// Trace recording commands (asciicast v2 format)
    Trace {
        #[command(subcommand)]
        action: TraceCommands,
    },
    /// Snapshot save and diff commands
    Snapshot {
        #[command(subcommand)]
        action: SnapshotCommands,
    },
}

#[derive(Subcommand)]
enum TraceCommands {
    /// Start recording an asciicast v2 trace
    Start {
        /// Session ID
        session: String,
        /// Output file path (defaults to /tmp/tui-wright-trace-<pid>.cast)
        #[arg(long)]
        output: Option<String>,
    },
    /// Stop recording and finalize the trace file
    Stop {
        /// Session ID
        session: String,
    },
    /// Insert a named marker into the trace
    Marker {
        /// Session ID
        session: String,
        /// Marker label
        label: String,
    },
}

#[derive(Subcommand)]
enum SnapshotCommands {
    /// Save current screen snapshot to a JSON file
    Save {
        /// Session ID
        session: String,
        /// Output file path
        file: String,
    },
    /// Compare current screen against a saved baseline (exit 0 if identical, 1 if different)
    Diff {
        /// Session ID
        session: String,
        /// Path to baseline JSON file
        file: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Spawn { command, args, cols, rows } => {
            let session_id = server::generate_session_id();
            let sock = server::socket_path(&session_id);
            let cwd = std::env::current_dir().expect("Failed to get current directory");

            // Fork to background using double-fork technique
            unsafe {
                let pid = libc::fork();
                if pid < 0 {
                    eprintln!("Failed to fork");
                    std::process::exit(1);
                }
                if pid > 0 {
                    // Parent: wait briefly for socket to appear, then print session ID
                    for _ in 0..50 {
                        if sock.exists() {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                    println!("session: {}", session_id);
                    return;
                }

                // First child: create new session and fork again
                libc::setsid();
                let pid2 = libc::fork();
                if pid2 < 0 {
                    std::process::exit(1);
                }
                if pid2 > 0 {
                    // First child exits immediately
                    std::process::exit(0);
                }

                // Grandchild: this is the daemon — redirect stdio to /dev/null
                let devnull = libc::open(b"/dev/null\0".as_ptr() as *const _, libc::O_RDWR);
                if devnull >= 0 {
                    libc::dup2(devnull, 0);
                    libc::dup2(devnull, 1);
                    libc::dup2(devnull, 2);
                    if devnull > 2 {
                        libc::close(devnull);
                    }
                }
            }

            if let Err(e) = server::run_daemon(&command, &args, cols, rows, &session_id, &cwd) {
                eprintln!("Daemon error: {}", e);
                let _ = std::fs::remove_file(&sock);
                std::process::exit(1);
            }
        }

        Commands::Screen { session, json } => {
            let request = Request::Screen { json };
            match client::send_request(&session, &request) {
                Ok(resp) => client::print_response(&resp),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Type { session, text } => {
            let request = Request::Type { text };
            match client::send_request(&session, &request) {
                Ok(resp) => client::print_response(&resp),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Key { session, name } => {
            let request = Request::Key { name };
            match client::send_request(&session, &request) {
                Ok(resp) => client::print_response(&resp),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Mouse { session, action, col, row } => {
            let request = Request::Mouse { action, col, row };
            match client::send_request(&session, &request) {
                Ok(resp) => client::print_response(&resp),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Resize { session, cols, rows } => {
            let request = Request::Resize { cols, rows };
            match client::send_request(&session, &request) {
                Ok(resp) => client::print_response(&resp),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Cursor { session } => {
            let request = Request::Cursor;
            match client::send_request(&session, &request) {
                Ok(resp) => client::print_response(&resp),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Kill { session } => {
            let request = Request::Kill;
            match client::send_request(&session, &request) {
                Ok(resp) => client::print_response(&resp),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::List => {
            let sessions = client::list_sessions();
            if sessions.is_empty() {
                println!("No active sessions");
            } else {
                for s in sessions {
                    println!("{}", s);
                }
            }
        }

        Commands::WaitFor { session, text, timeout } => {
            let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout);
            loop {
                let request = Request::Screen { json: false };
                match client::send_request(&session, &request) {
                    Ok(Response::Text { text: screen }) => {
                        if screen.contains(&text) {
                            println!("{}", screen);
                            std::process::exit(0);
                        }
                    }
                    Ok(Response::Error { message }) => {
                        eprintln!("Error: {}", message);
                        std::process::exit(1);
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                    _ => {}
                }
                if std::time::Instant::now() >= deadline {
                    eprintln!("Timeout: \"{}\" not found after {}ms", text, timeout);
                    std::process::exit(1);
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }

        Commands::Assert { session, text } => {
            let request = Request::Screen { json: false };
            match client::send_request(&session, &request) {
                Ok(Response::Text { text: screen }) => {
                    println!("{}", screen);
                    if screen.contains(&text) {
                        std::process::exit(0);
                    } else {
                        std::process::exit(1);
                    }
                }
                Ok(Response::Error { message }) => {
                    eprintln!("Error: {}", message);
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
                _ => {
                    eprintln!("Unexpected response");
                    std::process::exit(1);
                }
            }
        }

        Commands::Run { command, cols, rows } => {
            let session_id = server::generate_session_id();
            let sock = server::socket_path(&session_id);
            let cwd = std::env::current_dir().expect("Failed to get current directory");

            unsafe {
                let pid = libc::fork();
                if pid < 0 {
                    eprintln!("Failed to fork");
                    std::process::exit(1);
                }
                if pid > 0 {
                    for _ in 0..50 {
                        if sock.exists() {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }

                    let type_req = Request::Type { text: command };
                    if let Err(e) = client::send_request(&session_id, &type_req) {
                        eprintln!("Error typing command: {}", e);
                        std::process::exit(1);
                    }
                    let key_req = Request::Key { name: "enter".to_string() };
                    if let Err(e) = client::send_request(&session_id, &key_req) {
                        eprintln!("Error sending enter: {}", e);
                        std::process::exit(1);
                    }

                    println!("session: {}", session_id);
                    return;
                }

                libc::setsid();
                let pid2 = libc::fork();
                if pid2 < 0 {
                    std::process::exit(1);
                }
                if pid2 > 0 {
                    std::process::exit(0);
                }

                let devnull = libc::open(b"/dev/null\0".as_ptr() as *const _, libc::O_RDWR);
                if devnull >= 0 {
                    libc::dup2(devnull, 0);
                    libc::dup2(devnull, 1);
                    libc::dup2(devnull, 2);
                    if devnull > 2 {
                        libc::close(devnull);
                    }
                }
            }

            if let Err(e) = server::run_daemon("bash", &[], cols, rows, &session_id, &cwd) {
                eprintln!("Daemon error: {}", e);
                let _ = std::fs::remove_file(&sock);
                std::process::exit(1);
            }
        }

        Commands::Trace { action } => match action {
            TraceCommands::Start { session, output } => {
                let request = Request::TraceStart { output };
                match client::send_request(&session, &request) {
                    Ok(resp) => client::print_response(&resp),
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            TraceCommands::Stop { session } => {
                let request = Request::TraceStop;
                match client::send_request(&session, &request) {
                    Ok(resp) => client::print_response(&resp),
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            TraceCommands::Marker { session, label } => {
                let request = Request::TraceMarker { label };
                match client::send_request(&session, &request) {
                    Ok(resp) => client::print_response(&resp),
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        },

        Commands::Snapshot { action } => match action {
            SnapshotCommands::Save { session, file } => {
                let request = Request::Screen { json: true };
                match client::send_request(&session, &request) {
                    Ok(Response::Screen { snapshot }) => {
                        let json = serde_json::to_string_pretty(&snapshot).unwrap();
                        if let Err(e) = std::fs::write(&file, json) {
                            eprintln!("Error writing file: {}", e);
                            std::process::exit(1);
                        }
                        println!("Snapshot saved to {}", file);
                    }
                    Ok(Response::Error { message }) => {
                        eprintln!("Error: {}", message);
                        std::process::exit(1);
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                    _ => {
                        eprintln!("Unexpected response");
                        std::process::exit(1);
                    }
                }
            }
            SnapshotCommands::Diff { session, file } => {
                let content = match std::fs::read_to_string(&file) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Error reading baseline file: {}", e);
                        std::process::exit(1);
                    }
                };
                let baseline: tui_wright::screen::ScreenSnapshot = match serde_json::from_str(&content) {
                    Ok(b) => b,
                    Err(e) => {
                        eprintln!("Error parsing baseline JSON: {}", e);
                        std::process::exit(1);
                    }
                };

                let request = Request::SnapshotDiff { baseline };
                match client::send_request(&session, &request) {
                    Ok(Response::Diff { diff }) => {
                        let json = serde_json::to_string_pretty(&diff).unwrap();
                        println!("{}", json);
                        if diff.identical {
                            std::process::exit(0);
                        } else {
                            std::process::exit(1);
                        }
                    }
                    Ok(Response::Error { message }) => {
                        eprintln!("Error: {}", message);
                        std::process::exit(1);
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                    _ => {
                        eprintln!("Unexpected response");
                        std::process::exit(1);
                    }
                }
            }
        },
    }
}

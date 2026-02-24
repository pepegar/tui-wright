use clap::{Parser, Subcommand};

use tui_wright::client;
use tui_wright::protocol::Request;
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
    }
}

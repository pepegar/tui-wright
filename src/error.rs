use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("PTY error: {0}")]
    Pty(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Unknown key name: {0}")]
    UnknownKey(String),

    #[error("Unknown mouse action: {0}")]
    UnknownMouseAction(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Child process exited")]
    ChildExited,
}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Error::Pty(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

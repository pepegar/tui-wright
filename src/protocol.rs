use serde::{Deserialize, Serialize};

use crate::diff::SnapshotDiff;
use crate::screen::ScreenSnapshot;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Request {
    Screen { json: bool },
    Type { text: String },
    Key { name: String },
    Mouse { action: String, col: u16, row: u16 },
    Resize { cols: u16, rows: u16 },
    Cursor,
    Kill,
    TraceStart { output: Option<String> },
    TraceStop,
    TraceMarker { label: String },
    SnapshotDiff { baseline: ScreenSnapshot },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Response {
    Ok,
    Text { text: String },
    Screen { snapshot: ScreenSnapshot },
    Cursor { row: u16, col: u16 },
    Error { message: String },
    Diff { diff: SnapshotDiff },
}

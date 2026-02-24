use serde::{Deserialize, Serialize};

use crate::screen::{CellInfo, ColorInfo, ScreenSnapshot};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDiff {
    pub identical: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions_changed: Option<DimensionChange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor_changed: Option<CursorChange>,
    pub changed_cells: Vec<CellChange>,
    pub summary: DiffSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionChange {
    pub old_rows: u16,
    pub old_cols: u16,
    pub new_rows: u16,
    pub new_cols: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorChange {
    pub old_row: u16,
    pub old_col: u16,
    pub new_row: u16,
    pub new_col: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellChange {
    pub row: u16,
    pub col: u16,
    pub old: CellInfo,
    pub new: CellInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    pub total_cells_compared: usize,
    pub changed_cell_count: usize,
    pub dimensions_match: bool,
    pub cursor_matches: bool,
}

fn empty_cell() -> CellInfo {
    CellInfo {
        char: " ".to_string(),
        fg: ColorInfo { r: 255, g: 255, b: 255 },
        bg: ColorInfo { r: 0, g: 0, b: 0 },
        bold: false,
        italic: false,
        underline: false,
        inverse: false,
    }
}

pub fn compute_diff(baseline: &ScreenSnapshot, current: &ScreenSnapshot) -> SnapshotDiff {
    let dimensions_changed = if baseline.rows != current.rows || baseline.cols != current.cols {
        Some(DimensionChange {
            old_rows: baseline.rows,
            old_cols: baseline.cols,
            new_rows: current.rows,
            new_cols: current.cols,
        })
    } else {
        None
    };

    let cursor_changed = if baseline.cursor_row != current.cursor_row
        || baseline.cursor_col != current.cursor_col
    {
        Some(CursorChange {
            old_row: baseline.cursor_row,
            old_col: baseline.cursor_col,
            new_row: current.cursor_row,
            new_col: current.cursor_col,
        })
    } else {
        None
    };

    let mut changed_cells = Vec::new();
    let compare_rows = baseline.rows.min(current.rows) as usize;
    let compare_cols = baseline.cols.min(current.cols) as usize;

    for row in 0..compare_rows {
        for col in 0..compare_cols {
            let old_cell = &baseline.cells[row][col];
            let new_cell = &current.cells[row][col];
            if old_cell != new_cell {
                changed_cells.push(CellChange {
                    row: row as u16,
                    col: col as u16,
                    old: old_cell.clone(),
                    new: new_cell.clone(),
                });
            }
        }
    }

    for row in compare_rows..current.rows as usize {
        for col in 0..current.cols as usize {
            changed_cells.push(CellChange {
                row: row as u16,
                col: col as u16,
                old: empty_cell(),
                new: current.cells[row][col].clone(),
            });
        }
    }

    for row in 0..compare_rows {
        for col in compare_cols..current.cols as usize {
            changed_cells.push(CellChange {
                row: row as u16,
                col: col as u16,
                old: empty_cell(),
                new: current.cells[row][col].clone(),
            });
        }
    }

    for row in compare_rows..baseline.rows as usize {
        for col in 0..baseline.cols as usize {
            changed_cells.push(CellChange {
                row: row as u16,
                col: col as u16,
                old: baseline.cells[row][col].clone(),
                new: empty_cell(),
            });
        }
    }

    for row in 0..compare_rows {
        for col in compare_cols..baseline.cols as usize {
            changed_cells.push(CellChange {
                row: row as u16,
                col: col as u16,
                old: baseline.cells[row][col].clone(),
                new: empty_cell(),
            });
        }
    }

    let total_cells = (baseline.rows.max(current.rows) as usize)
        * (baseline.cols.max(current.cols) as usize);
    let identical =
        dimensions_changed.is_none() && cursor_changed.is_none() && changed_cells.is_empty();

    let summary = DiffSummary {
        total_cells_compared: total_cells,
        changed_cell_count: changed_cells.len(),
        dimensions_match: dimensions_changed.is_none(),
        cursor_matches: cursor_changed.is_none(),
    };

    SnapshotDiff {
        identical,
        dimensions_changed,
        cursor_changed,
        changed_cells,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screen;

    #[test]
    fn test_identical_snapshots() {
        let mut parser = vt100::Parser::new(4, 10, 0);
        parser.process(b"hello");
        let snap = screen::from_screen(parser.screen());
        let diff = compute_diff(&snap, &snap);
        assert!(diff.identical);
        assert!(diff.dimensions_changed.is_none());
        assert!(diff.cursor_changed.is_none());
        assert_eq!(diff.changed_cells.len(), 0);
        assert_eq!(diff.summary.changed_cell_count, 0);
    }

    #[test]
    fn test_text_change() {
        let mut parser1 = vt100::Parser::new(4, 10, 0);
        parser1.process(b"hello");
        let snap1 = screen::from_screen(parser1.screen());

        let mut parser2 = vt100::Parser::new(4, 10, 0);
        parser2.process(b"world");
        let snap2 = screen::from_screen(parser2.screen());

        let diff = compute_diff(&snap1, &snap2);
        assert!(!diff.identical);
        assert!(diff.dimensions_changed.is_none());
        assert!(diff.changed_cells.len() > 0);
    }

    #[test]
    fn test_cursor_change() {
        let mut parser1 = vt100::Parser::new(4, 10, 0);
        parser1.process(b"ab");
        let snap1 = screen::from_screen(parser1.screen());

        let mut parser2 = vt100::Parser::new(4, 10, 0);
        parser2.process(b"abcd");
        let snap2 = screen::from_screen(parser2.screen());

        let diff = compute_diff(&snap1, &snap2);
        assert!(!diff.identical);
        assert!(diff.cursor_changed.is_some());
        let cursor = diff.cursor_changed.unwrap();
        assert_eq!(cursor.old_col, 2);
        assert_eq!(cursor.new_col, 4);
    }

    #[test]
    fn test_dimension_change() {
        let mut parser1 = vt100::Parser::new(4, 10, 0);
        parser1.process(b"test");
        let snap1 = screen::from_screen(parser1.screen());

        let mut parser2 = vt100::Parser::new(6, 12, 0);
        parser2.process(b"test");
        let snap2 = screen::from_screen(parser2.screen());

        let diff = compute_diff(&snap1, &snap2);
        assert!(!diff.identical);
        assert!(diff.dimensions_changed.is_some());
        let dims = diff.dimensions_changed.unwrap();
        assert_eq!(dims.old_rows, 4);
        assert_eq!(dims.old_cols, 10);
        assert_eq!(dims.new_rows, 6);
        assert_eq!(dims.new_cols, 12);
    }

    #[test]
    fn test_diff_serialization() {
        let mut parser = vt100::Parser::new(4, 10, 0);
        parser.process(b"hello");
        let snap1 = screen::from_screen(parser.screen());

        parser.process(b" world");
        let snap2 = screen::from_screen(parser.screen());

        let diff = compute_diff(&snap1, &snap2);
        let json = serde_json::to_string(&diff).unwrap();
        let _: SnapshotDiff = serde_json::from_str(&json).unwrap();
    }
}

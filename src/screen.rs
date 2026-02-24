use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenSnapshot {
    pub rows: u16,
    pub cols: u16,
    pub cursor_row: u16,
    pub cursor_col: u16,
    pub cells: Vec<Vec<CellInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellInfo {
    pub char: String,
    pub fg: ColorInfo,
    pub bg: ColorInfo,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorInfo {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl ColorInfo {
    pub fn from_vt100_color(color: vt100::Color) -> Self {
        match color {
            vt100::Color::Default => ColorInfo { r: 255, g: 255, b: 255 },
            vt100::Color::Idx(idx) => idx_to_rgb(idx),
            vt100::Color::Rgb(r, g, b) => ColorInfo { r, g, b },
        }
    }

    pub fn from_vt100_bg(color: vt100::Color) -> Self {
        match color {
            vt100::Color::Default => ColorInfo { r: 0, g: 0, b: 0 },
            vt100::Color::Idx(idx) => idx_to_rgb(idx),
            vt100::Color::Rgb(r, g, b) => ColorInfo { r, g, b },
        }
    }
}

fn idx_to_rgb(idx: u8) -> ColorInfo {
    static BASIC: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (205, 0, 0),
        (0, 205, 0),
        (205, 205, 0),
        (0, 0, 238),
        (205, 0, 205),
        (0, 205, 205),
        (229, 229, 229),
        (127, 127, 127),
        (255, 0, 0),
        (0, 255, 0),
        (255, 255, 0),
        (92, 92, 255),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
    ];

    if idx < 16 {
        let (r, g, b) = BASIC[idx as usize];
        return ColorInfo { r, g, b };
    }

    if idx < 232 {
        let idx = idx - 16;
        let r = (idx / 36) * 51;
        let g = ((idx % 36) / 6) * 51;
        let b = (idx % 6) * 51;
        return ColorInfo { r, g, b };
    }

    let gray = 8 + (idx - 232) * 10;
    ColorInfo { r: gray, g: gray, b: gray }
}

pub fn from_screen(screen: &vt100::Screen) -> ScreenSnapshot {
    let size = screen.size();
    let (rows, cols) = (size.0, size.1);
    let cursor = screen.cursor_position();

    let mut cells = Vec::with_capacity(rows as usize);
    for row in 0..rows {
        let mut row_cells = Vec::with_capacity(cols as usize);
        for col in 0..cols {
            let cell = screen.cell(row, col).unwrap();
            row_cells.push(CellInfo {
                char: cell.contents(),
                fg: ColorInfo::from_vt100_color(cell.fgcolor()),
                bg: ColorInfo::from_vt100_bg(cell.bgcolor()),
                bold: cell.bold(),
                italic: cell.italic(),
                underline: cell.underline(),
                inverse: cell.inverse(),
            });
        }
        cells.push(row_cells);
    }

    ScreenSnapshot {
        rows,
        cols,
        cursor_row: cursor.0,
        cursor_col: cursor.1,
        cells,
    }
}

pub fn screen_text(screen: &vt100::Screen) -> String {
    let size = screen.size();
    let mut lines = Vec::new();
    for row in 0..size.0 {
        let mut line = String::new();
        for col in 0..size.1 {
            if let Some(cell) = screen.cell(row, col) {
                let contents = cell.contents();
                if contents.is_empty() {
                    line.push(' ');
                } else {
                    line.push_str(&contents);
                }
            }
        }
        lines.push(line.trim_end().to_string());
    }

    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idx_to_rgb_basic() {
        let c = idx_to_rgb(0);
        assert_eq!((c.r, c.g, c.b), (0, 0, 0));

        let c = idx_to_rgb(1);
        assert_eq!((c.r, c.g, c.b), (205, 0, 0));

        let c = idx_to_rgb(15);
        assert_eq!((c.r, c.g, c.b), (255, 255, 255));
    }

    #[test]
    fn test_idx_to_rgb_grayscale() {
        let c = idx_to_rgb(232);
        assert_eq!((c.r, c.g, c.b), (8, 8, 8));

        let c = idx_to_rgb(255);
        assert_eq!((c.r, c.g, c.b), (238, 238, 238));
    }

    #[test]
    fn test_from_screen() {
        let mut parser = vt100::Parser::new(24, 80, 0);
        parser.process(b"Hello, world!");
        let snap = from_screen(parser.screen());
        assert_eq!(snap.rows, 24);
        assert_eq!(snap.cols, 80);
        assert_eq!(snap.cells[0][0].char, "H");
        assert_eq!(snap.cells[0][4].char, "o");
    }

    #[test]
    fn test_screen_text() {
        let mut parser = vt100::Parser::new(24, 80, 0);
        parser.process(b"Hello, world!");
        let text = screen_text(parser.screen());
        assert!(text.starts_with("Hello, world!"));
    }

    #[test]
    fn test_snapshot_serialization() {
        let mut parser = vt100::Parser::new(4, 10, 0);
        parser.process(b"test");
        let snap = from_screen(parser.screen());
        let json = serde_json::to_string(&snap).unwrap();
        let _: ScreenSnapshot = serde_json::from_str(&json).unwrap();
    }
}

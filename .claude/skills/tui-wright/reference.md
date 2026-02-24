# tui-wright Reference

## JSON Screen Schema

The `tui-wright screen <session> --json` command returns a JSON object with the following structure:

```json
{
  "rows": 24,
  "cols": 80,
  "cursor_row": 5,
  "cursor_col": 12,
  "cells": [
    [
      {
        "char": "H",
        "fg": { "r": 255, "g": 255, "b": 255 },
        "bg": { "r": 0, "g": 0, "b": 0 },
        "bold": false,
        "italic": false,
        "underline": false,
        "inverse": false
      }
    ]
  ]
}
```

### Top-level fields

| Field | Type | Description |
|-------|------|-------------|
| `rows` | `u16` | Number of rows in the terminal |
| `cols` | `u16` | Number of columns in the terminal |
| `cursor_row` | `u16` | Current cursor row (0-indexed) |
| `cursor_col` | `u16` | Current cursor column (0-indexed) |
| `cells` | `CellInfo[][]` | 2D array: `cells[row][col]` |

### CellInfo fields

| Field | Type | Description |
|-------|------|-------------|
| `char` | `string` | The character at this cell (empty string for blank cells) |
| `fg` | `ColorInfo` | Foreground color |
| `bg` | `ColorInfo` | Background color |
| `bold` | `bool` | Whether the cell is bold |
| `italic` | `bool` | Whether the cell is italic |
| `underline` | `bool` | Whether the cell is underlined |
| `inverse` | `bool` | Whether fg/bg are swapped |

### ColorInfo fields

| Field | Type | Description |
|-------|------|-------------|
| `r` | `u8` | Red component (0-255) |
| `g` | `u8` | Green component (0-255) |
| `b` | `u8` | Blue component (0-255) |

Default foreground is `{r: 255, g: 255, b: 255}` (white). Default background is `{r: 0, g: 0, b: 0}` (black).

## Useful jq Recipes

```bash
# Get cursor position from JSON
tui-wright screen $SESSION --json | jq '{row: .cursor_row, col: .cursor_col}'

# Extract text from a specific row (row 0)
tui-wright screen $SESSION --json | jq '[.cells[0][].char] | join("")'

# Extract text from rows 5-10
tui-wright screen $SESSION --json | jq '[.cells[5:11][] | [.[].char] | join("")] | .[]'

# Find all bold cells
tui-wright screen $SESSION --json | jq '[range(.rows) as $r | range(.cols) as $c | .cells[$r][$c] | select(.bold) | {row: $r, col: $c, char}]'

# Check if a specific cell has a particular color
tui-wright screen $SESSION --json | jq '.cells[0][0].fg'

# Get the dimensions
tui-wright screen $SESSION --json | jq '{rows, cols}'
```

## Spawn Options

| Option | Default | Description |
|--------|---------|-------------|
| `--cols <N>` | 80 | Terminal width in columns |
| `--rows <N>` | 24 | Terminal height in rows |

## Mouse Actions

| Action | Description |
|--------|-------------|
| `press` / `click` | Mouse button press |
| `release` | Mouse button release |
| `move` | Mouse movement (for hover or drag) |
| `scrollup` / `scroll-up` | Scroll wheel up |
| `scrolldown` / `scroll-down` | Scroll wheel down |

Coordinates are 0-indexed (`col`, `row`). Uses SGR mouse encoding so there is no 223-column limit.

For a full click event, send both `press` and `release`:

```bash
tui-wright mouse $SESSION press 10 5
tui-wright mouse $SESSION release 10 5
```

## Error Handling

Commands exit with code 0 on success. On failure, an error message is printed to stderr and the exit code is 1.

Common errors:
- **Session not found** — the session ID doesn't match any running daemon (typo, or it was already killed)
- **Unknown key name** — the key name passed to `key` wasn't recognized
- **Unknown mouse action** — the action passed to `mouse` wasn't recognized
- **Connection refused** — the daemon process crashed or was killed externally

## Architecture Notes

Each `spawn` command starts a background daemon process (double-forked, detached from the terminal). The daemon:
1. Opens a PTY via `portable-pty`
2. Spawns the child command in the PTY
3. Runs a reader thread feeding PTY output into `vt100::Parser`
4. Listens on a Unix domain socket at `$TMPDIR/tui-wright-<session-id>.sock`

All other commands (`screen`, `type`, `key`, etc.) are thin clients that:
1. Connect to the Unix socket
2. Send a JSON request
3. Receive a JSON response
4. Print the result and exit

This means sessions persist across separate Bash invocations — you can `spawn` in one call and `screen` in the next.

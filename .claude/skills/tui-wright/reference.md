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

## Python Parsing Recipes

For more complex analysis, Python is often easier than jq:

```bash
tui-wright screen $SESSION --json | python3 -c "
import json, sys
data = json.load(sys.stdin)

# Extract text from a specific row
row = data['cells'][0]
text = ''.join(c.get('char', ' ') for c in row)
print(text.strip())

# Find cells with specific attributes
for r, row in enumerate(data['cells']):
    for c, cell in enumerate(row):
        if cell.get('bold'):
            ch = cell.get('char', ' ')
            fg = cell.get('fg', {})
            print('row=%d col=%d char=%r fg=(%d,%d,%d)' % (
                r, c, ch, fg.get('r',0), fg.get('g',0), fg.get('b',0)))

# Find a row containing specific text
for i, row in enumerate(data['cells']):
    line = ''.join(c.get('char', ' ') for c in row)
    if 'search term' in line:
        print('Row %d: %s' % (i, line.strip()))
"
```

**Note:** Avoid `!r` inside f-strings when using inline Python with bash — use `%` formatting or intermediate variables instead.

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

## Snapshot Diff Schema

The `tui-wright snapshot diff <session> <file>` command returns a JSON object:

```json
{
  "identical": false,
  "dimensions_changed": {
    "old_rows": 24,
    "old_cols": 80,
    "new_rows": 40,
    "new_cols": 120
  },
  "cursor_changed": {
    "old_row": 0,
    "old_col": 5,
    "new_row": 2,
    "new_col": 10
  },
  "changed_cells": [
    {
      "row": 0,
      "col": 0,
      "old": { "char": "H", "fg": {"r":255,"g":255,"b":255}, "bg": {"r":0,"g":0,"b":0}, "bold": false, "italic": false, "underline": false, "inverse": false },
      "new": { "char": "W", "fg": {"r":255,"g":255,"b":255}, "bg": {"r":0,"g":0,"b":0}, "bold": false, "italic": false, "underline": false, "inverse": false }
    }
  ],
  "summary": {
    "total_cells_compared": 1920,
    "changed_cell_count": 5,
    "dimensions_match": false,
    "cursor_matches": false
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `identical` | `bool` | `true` if screens are exactly the same |
| `dimensions_changed` | `object?` | Present only if rows/cols differ between baseline and current |
| `cursor_changed` | `object?` | Present only if cursor position moved |
| `changed_cells` | `CellChange[]` | Every cell that differs (char, color, or style) |
| `summary` | `object` | Counts and boolean flags for quick inspection |

Exit code: **0** if `identical` is true, **1** if false.

## Trace Recording Format (asciicast v2)

`tui-wright trace start` records a `.cast` file in the [asciicast v2](https://docs.asciinema.org/manual/asciicast/v2/) format — newline-delimited JSON (NDJSON).

```
{"version":2,"width":80,"height":24,"timestamp":1740000000}
[0.0,"o","$ echo hello\r\n"]
[0.05,"i","echo hello"]
[0.06,"m","type \"echo hello\""]
[0.07,"i","\r"]
[0.08,"m","key enter"]
[0.5,"o","hello\r\n$ "]
[1.0,"m","after-setup"]
[2.0,"r","120x40"]
```

**Line 1** is the header (JSON object). **Lines 2+** are events (JSON arrays: `[time, code, data]`).

| Event code | Description | Data format |
|------------|-------------|-------------|
| `"o"` | Terminal output | Raw PTY bytes as UTF-8 string |
| `"i"` | Input sent to terminal | Raw bytes sent by tui-wright |
| `"m"` | Marker / chapter point | Label string |
| `"r"` | Terminal resize | `"COLSxROWS"` string |

Auto-generated markers are inserted for every Key, Type, and Mouse command (e.g., `"key enter"`, `"type \"ls -la\""`, `"mouse press 10,5"`).

Play back with:

```bash
asciinema play recording.cast
```

Or embed in HTML with the [asciinema-player](https://docs.asciinema.org/manual/player/quick-start/).

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

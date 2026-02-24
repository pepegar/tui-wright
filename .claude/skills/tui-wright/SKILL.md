---
name: tui-wright
description: Use this skill when you need to interact with a TUI (terminal UI) application programmatically — spawning it, reading its screen, sending keyboard/mouse input, resizing, or debugging its behavior. Use when the user asks to "debug a TUI", "interact with a terminal app", "test a CLI tool", "read what's on screen", or anything involving programmatic control of a terminal application.
---

# tui-wright: Programmatic TUI Control

You have access to `tui-wright`, a tool that spawns TUI processes in a virtual terminal and lets you control them via separate CLI commands. This is how you interact with terminal applications like `htop`, `vim`, `less`, `psql`, database REPLs, or any program that draws to the terminal.

## Quick Reference

```bash
tui-wright spawn <command> [args...]   # Start a session (returns session ID)
tui-wright run <command>               # Spawn a shell, type command, press enter (returns session ID)
tui-wright screen <session>            # Read current screen text
tui-wright screen <session> --json     # Screen with cell attributes (color, bold, etc.)
tui-wright type <session> <text>       # Send text characters
tui-wright key <session> <name>        # Send special key
tui-wright mouse <session> <action> <col> <row>  # Send mouse event
tui-wright resize <session> <cols> <rows>         # Change viewport
tui-wright cursor <session>            # Get cursor position
tui-wright waitfor <session> <text>    # Wait until text appears on screen (or timeout)
tui-wright assert <session> <text>     # Assert text is visible (exit 0 if found, 1 if not)
tui-wright kill <session>              # End session
tui-wright list                        # List active sessions

# Trace recording (asciicast v2 format)
tui-wright trace start <session> [--output path.cast]  # Start recording
tui-wright trace stop <session>                        # Stop recording, finalize .cast file
tui-wright trace marker <session> <label>              # Insert a named marker

# Snapshot diffing
tui-wright snapshot save <session> <file>   # Save current screen as JSON baseline
tui-wright snapshot diff <session> <file>   # Compare current screen to baseline (exit 0=identical, 1=different)
```

## Core Workflow

Every interaction follows the same pattern: **act, wait, read**.

```bash
# 1. Spawn the application
SESSION=$(tui-wright spawn <command> | awk '{print $2}')

# 2. Do something (type, key, mouse, resize)
tui-wright type $SESSION "some input"
tui-wright key $SESSION enter

# 3. Wait for the TUI to update (important!)
sleep 0.2

# 4. Read the screen to see what happened
tui-wright screen $SESSION
```

**Always add a short sleep** (0.2-0.5s) after sending input before reading the screen. TUI applications need time to process input and redraw.

**Prefer `waitfor` over `sleep`** when you know what text should appear — it's both faster and more reliable:

```bash
tui-wright key $SESSION enter
tui-wright waitfor $SESSION "expected output"   # returns as soon as text appears (up to 5s)
```

## Spawning Sessions

```bash
# Default: 80x24 terminal
SESSION=$(tui-wright spawn bash | awk '{print $2}')

# Custom size
SESSION=$(tui-wright spawn htop --cols 120 --rows 40 | awk '{print $2}')

# With arguments
SESSION=$(tui-wright spawn vim myfile.txt | awk '{print $2}')
```

The `spawn` command starts a background daemon and returns immediately. The session ID is a short hex string like `a1b2c3`.

### Running a command directly

If you just want to run a command in a shell (spawn + type + enter in one step), use `run`:

```bash
# Spawns a bash shell, types the command, and presses enter
SESSION=$(tui-wright run "ls -la /tmp" | awk '{print $2}')
sleep 0.3
tui-wright screen $SESSION

# With custom terminal size
SESSION=$(tui-wright run "htop" --cols 120 --rows 40 | awk '{print $2}')
```

This is equivalent to `spawn bash` followed by `type` and `key enter`, but in a single command.

## Sending Input

### Text

```bash
tui-wright type $SESSION "SELECT * FROM users;"
```

This sends characters one by one, as if typed on a keyboard. Does **not** add a newline — use `key enter` for that.

### Special Keys

```bash
tui-wright key $SESSION enter
tui-wright key $SESSION tab
tui-wright key $SESSION escape
tui-wright key $SESSION up
tui-wright key $SESSION ctrl+c
tui-wright key $SESSION f5
```

Supported key names:

| Category | Keys |
|----------|------|
| Navigation | `up`, `down`, `left`, `right`, `home`, `end`, `pageup`/`pgup`, `pagedown`/`pgdn` |
| Editing | `enter`/`return`, `tab`, `backspace`/`bs`, `delete`/`del`, `insert`/`ins`, `space` |
| Modifiers | `ctrl+a` through `ctrl+z`, `alt+<char>` |
| Function | `f1` through `f12` |
| Other | `escape`/`esc` |

Both `ctrl+c` and `ctrl-c` syntax work. Key names are case-insensitive.

**`shift+<key>` is NOT supported.** To send uppercase letters (e.g., htop's `M` for sort-by-memory), use `type` instead of `key`:

```bash
# WRONG — will error with "Unknown key name: shift+m"
tui-wright key $SESSION shift+m

# CORRECT — sends the uppercase character 'M'
tui-wright type $SESSION "M"
```

This applies to any shortcut that uses an uppercase letter.

### Mouse

```bash
tui-wright mouse $SESSION press 10 5      # Click at column 10, row 5
tui-wright mouse $SESSION release 10 5    # Release at same position
tui-wright mouse $SESSION scrollup 0 0    # Scroll up
tui-wright mouse $SESSION scrolldown 0 0  # Scroll down
tui-wright mouse $SESSION move 15 8       # Mouse move (for hover/drag)
```

Coordinates are 0-indexed. Uses SGR mouse encoding (no column limit).

## Reading the Screen

### Plain text (most common)

```bash
tui-wright screen $SESSION
```

Returns the terminal content as plain text, one line per row, trailing whitespace trimmed, trailing empty lines removed.

### JSON with cell attributes

```bash
tui-wright screen $SESSION --json
```

Returns a JSON object. See [reference.md](./reference.md) for the full schema.

Use `--json` when you need to inspect colors, bold/italic styling, or precise cell contents. For most debugging, plain text is sufficient.

### Cursor position

```bash
tui-wright cursor $SESSION
# Output: row: 5, col: 12
```

## Resizing

```bash
tui-wright resize $SESSION 120 40
```

The TUI application receives a `SIGWINCH` and redraws at the new size. Wait briefly after resizing before reading the screen.

## Waiting and Assertions

### Wait for text to appear

`waitfor` polls the screen until the given text appears, or times out:

```bash
tui-wright waitfor $SESSION "expected output"              # Default 5s timeout
tui-wright waitfor $SESSION "Loading complete" --timeout 10000  # 10s timeout
```

On success, prints the screen contents and exits 0. On timeout, prints an error to stderr and exits 1.

**Prefer `waitfor` over `sleep`** — it returns as soon as the text appears, so it's both faster and more reliable than guessing a sleep duration.

### Assert text is visible

`assert` checks whether text is currently on screen (no polling, no waiting):

```bash
tui-wright assert $SESSION "Welcome"    # exit 0 if found, exit 1 if not
```

Always prints the current screen contents. Use `assert` for quick checks after you've already waited for the screen to stabilize.

## Session Management

```bash
# List all active sessions
tui-wright list

# Kill a specific session
tui-wright kill $SESSION
```

**Always kill sessions when done.** Each session is a background daemon holding a PTY.

## Trace Recording

Record all terminal activity as an [asciicast v2](https://docs.asciinema.org/manual/asciicast/v2/) `.cast` file, playable with `asciinema play` or the embeddable web player.

```bash
# Start recording
tui-wright trace start $SESSION --output /tmp/my-session.cast

# Do your interactions...
tui-wright type $SESSION "echo hello"
tui-wright key $SESSION enter

# Insert custom markers for navigation in the player
tui-wright trace marker $SESSION "after-setup"

# Stop recording and finalize the file
tui-wright trace stop $SESSION
```

The trace captures:
- **Output events (`"o"`)** — raw PTY output bytes with timestamps, exactly as the terminal received them
- **Input events (`"i"`)** — every keystroke, text, and mouse event sent through tui-wright
- **Markers (`"m"`)** — automatically inserted for each Key/Type/Mouse command (e.g., `"key enter"`, `"type echo hello"`), plus any custom markers you insert
- **Resize events (`"r"`)** — when `resize` is called

If no `--output` path is given, the file is written to a temp directory. The trace is automatically stopped if the session is killed.

Play back the recording:

```bash
asciinema play /tmp/my-session.cast
```

## Snapshot Diffing

Save and compare screen snapshots for visual regression testing.

```bash
# Save a baseline snapshot
tui-wright snapshot save $SESSION baseline.json

# ... make changes ...

# Compare current screen against the baseline
tui-wright snapshot diff $SESSION baseline.json
```

`snapshot diff` outputs a JSON diff with:
- **`identical`** — `true`/`false` overall result
- **`dimensions_changed`** — old/new rows and cols (if changed)
- **`cursor_changed`** — old/new cursor position (if moved)
- **`changed_cells`** — list of every cell that differs, with old and new values (char, fg/bg colors, bold, italic, underline, inverse)
- **`summary`** — total cells compared, number changed

Exit codes: **0** if identical, **1** if different — use in test scripts:

```bash
tui-wright snapshot diff $SESSION baseline.json && echo "PASS" || echo "FAIL"
```

## Practical Patterns

### Run a command in bash and read output

```bash
SESSION=$(tui-wright run "ls -la /tmp" | awk '{print $2}')
tui-wright waitfor $SESSION "$"    # wait for prompt to return
tui-wright screen $SESSION
tui-wright kill $SESSION
```

### Navigate a menu-driven TUI

```bash
SESSION=$(tui-wright spawn htop | awk '{print $2}')
sleep 0.5
tui-wright screen $SESSION          # See initial state
tui-wright key $SESSION down         # Move selection down
tui-wright key $SESSION down
sleep 0.2
tui-wright screen $SESSION          # See updated selection
tui-wright key $SESSION f10          # Quit htop
tui-wright kill $SESSION
```

### Work with a REPL

```bash
SESSION=$(tui-wright spawn python3 | awk '{print $2}')
sleep 0.5
tui-wright type $SESSION "2 + 2"
tui-wright key $SESSION enter
sleep 0.2
tui-wright screen $SESSION          # Should show "4"
tui-wright key $SESSION ctrl+d      # Exit python
tui-wright kill $SESSION
```

### Edit a file in vim

```bash
SESSION=$(tui-wright spawn vim test.txt | awk '{print $2}')
sleep 0.5
tui-wright key $SESSION i            # Enter insert mode
tui-wright type $SESSION "Hello, world!"
tui-wright key $SESSION escape       # Back to normal mode
tui-wright type $SESSION ":wq"       # Write and quit
tui-wright key $SESSION enter
tui-wright kill $SESSION
```

### Profile a system with htop

```bash
SESSION=$(tui-wright spawn htop --cols 120 --rows 40 | awk '{print $2}')
sleep 0.5
tui-wright screen $SESSION                    # Read initial state (sorted by CPU%)

# Sort by memory usage — 'M' is uppercase, so use type, not key
tui-wright type $SESSION "M"
sleep 0.3
tui-wright screen $SESSION                    # Now sorted by MEM%

# Filter to specific processes
tui-wright key $SESSION f4                    # Open filter
sleep 0.2
tui-wright type $SESSION "docker"             # Type filter text
sleep 0.3
tui-wright screen $SESSION                    # Only Docker processes shown
tui-wright key $SESSION escape                # Clear filter

# Toggle tree view to see process hierarchy
tui-wright key $SESSION f5
sleep 0.3
tui-wright screen $SESSION                    # Tree view with ├─ └─ connectors

# Search for a specific process
tui-wright key $SESSION f3                    # Open search
tui-wright type $SESSION "chrome"
sleep 0.3
tui-wright screen $SESSION                    # Cursor jumps to first match
tui-wright key $SESSION escape                # Close search

tui-wright key $SESSION f10                   # Quit htop
tui-wright kill $SESSION
```

### Inspect screen attributes for color/style debugging

```bash
SESSION=$(tui-wright spawn bash | awk '{print $2}')
tui-wright type $SESSION "ls --color"
tui-wright key $SESSION enter
sleep 0.3
tui-wright screen $SESSION --json | jq '.cells[1][0]'
# Shows: { "char": "f", "fg": { "r": 0, "g": 255, "b": 0 }, "bold": true, ... }
tui-wright kill $SESSION
```

## Tips

### Chain commands in a single Bash call

Multiple `tui-wright` commands can be chained with `&&` to reduce round-trips. Only add a sleep before `screen` reads:

```bash
tui-wright key $SESSION f4 && sleep 0.2 && tui-wright type $SESSION "filter text" && sleep 0.3 && tui-wright screen $SESSION
```

### Sleep durations

- **0.5s** for initial spawn (TUI apps need time to start and draw the first frame)
- **0.3s** after typing text into interactive prompts (search bars, filters, REPLs)
- **0.2s** after single key presses in an already-running app

Complex TUI apps (htop, vim) generally need longer sleeps than simple ones (bash, python REPL).

## Important Notes

- **Prefer `waitfor` over sleep.** Use `tui-wright waitfor $SESSION "text"` when you know what to expect — it's faster and more reliable. Fall back to `sleep 0.2-0.5` only when there's no specific text to wait for.
- **Always kill sessions when done.** They are background daemons.
- **Screen text is trimmed.** Trailing whitespace per line and trailing empty lines are removed.
- **Type does not add newlines.** Use `tui-wright key $SESSION enter` after typing a command.
- **Sessions persist across your Bash calls.** The session ID is all you need to reconnect.
- **Use `type` for uppercase shortcuts, not `key shift+X`.** The `shift+` modifier is not supported for keys. Send uppercase characters via `type` instead.
- **Traces are asciicast v2.** The `.cast` files work with `asciinema play`, the web player, and asciinema.org.
- **Snapshot diff uses exit codes.** Exit 0 means identical, exit 1 means different — chain with `&&` or `||` in scripts.

For the full JSON screen schema, diff schema, and advanced usage, see [reference.md](./reference.md).

# ink

A fast, minimal terminal text editor, written in Rust.

ink aims to be what [micro](https://micro-editor.github.io/) is (an editor you can use right away, with the shortcuts you already know: Ctrl+C, Ctrl+V, Ctrl+Z, Ctrl+S) while staying small and quick: a single native binary with near-instant startup.

> **Status: early development.** The basics work (editing, selection, clipboard, undo, search and replace, syntax highlighting), but ink is young: keep backups of anything important.

## Installation

You need a recent Rust toolchain (1.88 or newer).

```bash
git clone https://github.com/polpo21/ink.git
cd ink
cargo install --path .
```

This builds an optimised binary and places it in `~/.cargo/bin/ink`.

## Usage

```bash
ink notes.txt    # open a file (it is created when you save, if it doesn't exist)
ink              # start with an empty buffer
ink --help       # usage
```

Press **F1** inside ink to see every shortcut. The status bar stays quiet: file name, a dot when there are unsaved changes, and on the right the language, line:column and the F1 reminder.

### Files

| Key | Action |
|---|---|
| Ctrl+S | Save (asks for a name if the file is new) |
| Ctrl+Q | Quit (press twice to discard unsaved changes) |

### Editing

| Key | Action |
|---|---|
| Ctrl+C | Copy the selection, or the whole line if nothing is selected |
| Ctrl+X | Cut the selection, or the whole line if nothing is selected |
| Ctrl+V | Paste (a copied whole line goes above the cursor line) |
| Ctrl+Z | Undo (word by word while typing) |
| Ctrl+Y / Ctrl+Shift+Z | Redo |
| Ctrl+A | Select all |
| Ctrl+Backspace / Ctrl+Delete | Delete the word before / after the cursor |
| Enter | New line, keeping the indentation |
| Tab | Insert a tab |

### Moving and selecting

| Key | Action |
|---|---|
| Arrow keys | Move the cursor |
| Ctrl+Left / Ctrl+Right | Move by word |
| Home / End | Start (indentation first) / end of line |
| Ctrl+Home / Ctrl+End | Start / end of file |
| Page Up / Page Down | Move by one screen |
| Ctrl+G | Go to line (`42`, or `42:7` for a column too) |
| Shift + any move | Extend the selection |
| Mouse | Click to place the cursor, drag to select, wheel to scroll |
| Esc | Clear the selection |

### Search and replace

| Key | Action |
|---|---|
| Ctrl+F | Find (case-insensitive unless you type an uppercase letter) |
| F3 | Find next |
| Ctrl+H | Replace: for each match, `y` replaces, `n` skips, `a` replaces every match in the file, `Esc` stops |

### Help

| Key | Action |
|---|---|
| F1 | Show or hide the shortcuts panel (arrows or wheel to scroll) |

Replacing everything is a single undo step.

### Syntax highlighting

Picked from the file name, or from the `#!` line for scripts: Rust, Python, JavaScript/TypeScript, C/C++, Go, Shell, Fish, Lua, SQL, CSS, TOML, YAML, JSON, KDL, INI-style config files and Markdown.

Colours come from your terminal's 16-colour palette, so ink follows whatever theme the terminal uses. The highlighter is built in and only re-scans the lines that changed, keeping startup instant.

The clipboard is the system one: `wl-copy`/`wl-paste` on Wayland, `xclip` on X11. Without them ink uses its own internal clipboard. Files keep their line endings (LF or CRLF) and are written in place, so symlinks stay symlinks.

## Configuration

ink reads `~/.config/ink/config.toml` (or `$XDG_CONFIG_HOME/ink/config.toml`). Every setting is optional, and without the file ink uses the defaults. A mistake in the file never stops ink from opening: it falls back to the defaults and shows the error in the status bar.

```toml
tab_width = 4          # columns a tab takes on screen
tabs_to_spaces = false # Tab inserts spaces instead of a tab character
auto_indent = true     # Enter keeps the current indentation
line_numbers = true
mouse = true           # false = use the terminal's own text selection
scroll_lines = 3       # lines per mouse wheel step

[colors]               # names follow the terminal theme; "#rrggbb" is fixed
keyword = "magenta"
comment = "darkgray"

[languages.python]     # per-language overrides
tabs_to_spaces = true
```

[`config.example.toml`](config.example.toml) lists every setting with its default.

## Roadmap

- [x] Open a file and move around it
- [x] Insert and delete text
- [x] Save (Ctrl+S)
- [x] Copy, cut and paste with the system clipboard
- [x] Undo and redo
- [x] Search
- [x] Mouse support
- [x] Replace
- [x] Go to line
- [x] Syntax highlighting
- [x] Configuration file

## Building from source

```bash
cargo build            # debug build
cargo build --release  # optimised build (LTO, stripped)
cargo clippy           # lints
cargo test             # unit tests
```

## License

[MIT](LICENSE)

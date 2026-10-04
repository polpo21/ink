# ink

A fast, minimal terminal text editor, written in Rust.

ink aims to be what [micro](https://micro-editor.github.io/) is (an editor you can use right away, with the shortcuts you already know: Ctrl+C, Ctrl+V, Ctrl+Z, Ctrl+S) while staying small and quick: a single native binary with near-instant startup.

> **Status: early development.** The basics work (editing, selection, clipboard, undo, search, save), but ink is young: keep backups of anything important.

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
```

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
| Shift + any move | Extend the selection |
| Mouse | Click to place the cursor, drag to select, wheel to scroll |
| Esc | Clear the selection |

### Search

| Key | Action |
|---|---|
| Ctrl+F | Find (case-insensitive unless you type an uppercase letter) |
| F3 | Find next |

The clipboard is the system one: `wl-copy`/`wl-paste` on Wayland, `xclip` on X11. Without them ink uses its own internal clipboard. Files keep their line endings (LF or CRLF) and are written in place, so symlinks stay symlinks.

## Roadmap

- [x] Open a file and move around it
- [x] Insert and delete text
- [x] Save (Ctrl+S)
- [x] Copy, cut and paste with the system clipboard
- [x] Undo and redo
- [x] Search
- [x] Mouse support
- [ ] Replace
- [ ] Go to line
- [ ] Syntax highlighting
- [ ] Configuration file

## Building from source

```bash
cargo build            # debug build
cargo build --release  # optimised build (LTO, stripped)
cargo clippy           # lints
cargo test             # unit tests
```

## License

[MIT](LICENSE)

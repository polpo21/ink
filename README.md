# ink

A fast, minimal terminal text editor, written in Rust.

ink aims to be what [micro](https://micro-editor.github.io/) is (an editor you can use right away, with familiar shortcuts) while staying small and quick: a single native binary with near-instant startup.

> **Status: early development.** ink can open a file and move around it, but it can't edit or save yet. Don't use it for real work.

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

| Key | Action |
|---|---|
| Arrow keys | Move the cursor |
| Home / End | Start / end of line |
| Ctrl+Q | Quit |

## Roadmap

- [x] Open a file and move around it
- [ ] Insert and delete text
- [ ] Save (Ctrl+S)
- [ ] Copy, cut and paste with the system clipboard
- [ ] Undo and redo
- [ ] Search and replace
- [ ] Mouse support
- [ ] Syntax highlighting
- [ ] Configuration file

## Building from source

```bash
cargo build            # debug build
cargo build --release  # optimised build (LTO, stripped)
cargo clippy           # lints
```

## License

[MIT](LICENSE)

//! ink: a fast, minimal terminal text editor.

mod editor;

use std::path::PathBuf;

use anyhow::Result;

fn main() -> Result<()> {
    let path = std::env::args_os().nth(1).map(PathBuf::from);
    let mut editor = editor::Editor::open(path)?;

    let mut terminal = ratatui::init();
    let result = editor.run(&mut terminal);
    ratatui::restore();
    result
}

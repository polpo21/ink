//! ink: a fast, minimal terminal text editor.

mod buffer;
mod clipboard;
mod editor;
mod highlight;
mod history;

use std::{io::stdout, path::PathBuf};

use anyhow::Result;
use ratatui::crossterm::{
    event::{
        DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::supports_keyboard_enhancement,
};

fn main() -> Result<()> {
    let path = std::env::args_os().nth(1).map(PathBuf::from);
    let mut editor = editor::Editor::open(path)?;

    let mut terminal = ratatui::init();
    // Lets terminals that support it report keys like Ctrl+Shift+Z unambiguously.
    let enhanced = matches!(supports_keyboard_enhancement(), Ok(true));
    execute!(stdout(), EnableBracketedPaste, EnableMouseCapture)?;
    if enhanced {
        execute!(
            stdout(),
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        )?;
    }
    let ratatui_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_modes(enhanced);
        ratatui_hook(info);
    }));

    let result = editor.run(&mut terminal);
    restore_modes(enhanced);
    ratatui::restore();
    result
}

fn restore_modes(enhanced: bool) {
    if enhanced {
        let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
    }
    let _ = execute!(stdout(), DisableMouseCapture, DisableBracketedPaste);
}

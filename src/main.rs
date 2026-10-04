//! ink: a fast, minimal terminal text editor.

mod buffer;
mod clipboard;
mod config;
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

const HELP: &str = "\
ink: a fast, minimal terminal text editor

Usage: ink [FILE]

Options:
  -h, --help     Show this help
  -V, --version  Show the version

Settings are read from ~/.config/ink/config.toml.
Inside the editor, the status bar shows the main shortcuts.";

fn main() -> Result<()> {
    let arg = std::env::args_os().nth(1);
    match arg.as_ref().and_then(|a| a.to_str()) {
        Some("-h" | "--help") => {
            println!("{HELP}");
            return Ok(());
        }
        Some("-V" | "--version") => {
            println!("ink {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        _ => {}
    }
    let (config, config_error) = config::Config::load();
    let mouse = config.mouse;
    let mut editor = editor::Editor::open(arg.map(PathBuf::from), config, config_error)?;

    let mut terminal = ratatui::init();
    // Lets terminals that support it report keys like Ctrl+Shift+Z unambiguously.
    let enhanced = matches!(supports_keyboard_enhancement(), Ok(true));
    execute!(stdout(), EnableBracketedPaste)?;
    if mouse {
        execute!(stdout(), EnableMouseCapture)?;
    }
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

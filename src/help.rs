//! The F1 shortcuts panel.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Paragraph},
};

const SECTIONS: &[(&str, &[(&str, &str)])] = &[
    (
        "Files",
        &[
            ("Ctrl+S", "Save (asks for a name if the file is new)"),
            ("Ctrl+Q", "Quit (twice to discard unsaved changes)"),
        ],
    ),
    (
        "Editing",
        &[
            ("Ctrl+C", "Copy the selection, or the whole line"),
            ("Ctrl+X", "Cut the selection, or the whole line"),
            ("Ctrl+V", "Paste"),
            ("Ctrl+Z", "Undo"),
            ("Ctrl+Y", "Redo (also Ctrl+Shift+Z)"),
            ("Ctrl+A", "Select all"),
            ("Ctrl+Backspace", "Delete the word before the cursor"),
            ("Ctrl+Delete", "Delete the word after the cursor"),
            ("Enter", "New line, keeping the indentation"),
        ],
    ),
    (
        "Moving and selecting",
        &[
            ("Arrows", "Move the cursor"),
            ("Ctrl+Left/Right", "Move by word"),
            ("Home / End", "Start / end of the line"),
            ("Ctrl+Home / End", "Start / end of the file"),
            ("PgUp / PgDn", "Move by one screen"),
            ("Ctrl+G", "Go to line (42, or 42:7 for a column)"),
            ("Shift + move", "Extend the selection"),
            ("Mouse", "Click, drag to select, wheel to scroll"),
            ("Esc", "Clear the selection"),
        ],
    ),
    (
        "Search and replace",
        &[
            ("Ctrl+F", "Find (lowercase = ignore case)"),
            ("F3", "Find next"),
            ("Ctrl+H", "Replace: y yes · n skip · a all · Esc stop"),
        ],
    ),
    ("Help", &[("F1", "Show or hide this panel")]),
];

const KEY_WIDTH: usize = 17;

fn lines(accent: Color, dim: Color) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for (i, (title, keys)) in SECTIONS.iter().enumerate() {
        if i > 0 {
            lines.push(Line::raw(""));
        }
        lines.push(Line::styled(
            format!(" {title}"),
            Style::new().fg(accent).add_modifier(Modifier::BOLD),
        ));
        for (key, action) in *keys {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("   {key:<KEY_WIDTH$}"),
                    Style::new().add_modifier(Modifier::BOLD),
                ),
                Span::styled(*action, Style::new().fg(dim)),
            ]));
        }
    }
    lines
}

/// Draws the panel centred in `area`. `scroll` is clamped to the content.
pub fn draw(frame: &mut Frame, area: Rect, scroll: &mut usize, accent: Color, dim: Color) {
    let lines = lines(accent, dim);
    let width = area.width.saturating_sub(2).min(66);
    let height = area.height.saturating_sub(2).min(lines.len() as u16 + 2);
    let rect = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );
    *scroll = (*scroll).min(
        lines
            .len()
            .saturating_sub(height.saturating_sub(2) as usize),
    );
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(dim))
        .title(Span::styled(
            " Shortcuts ",
            Style::new().fg(accent).add_modifier(Modifier::BOLD),
        ))
        .title_bottom(Span::styled(
            " ↑↓ scroll · Esc close ",
            Style::new().fg(dim),
        ));
    frame.render_widget(Clear, rect);
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .scroll((*scroll as u16, 0)),
        rect,
    );
}

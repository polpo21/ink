use std::{fs, io, path::PathBuf};

use anyhow::{Context, Result};
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    layout::{Constraint, Layout, Position},
    style::Style,
    text::Line,
    widgets::Paragraph,
};

/// The editor state: one buffer, a cursor and a scroll offset.
pub struct Editor {
    path: Option<PathBuf>,
    lines: Vec<String>,
    /// Cursor position as (line, column), in characters.
    cursor: (usize, usize),
    /// First visible line.
    scroll: usize,
    quit: bool,
}

impl Editor {
    pub fn open(path: Option<PathBuf>) -> Result<Self> {
        let lines = match &path {
            Some(p) => match fs::read_to_string(p) {
                Ok(text) => text.lines().map(str::to_owned).collect(),
                Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
                Err(e) => return Err(e).with_context(|| format!("cannot open {}", p.display())),
            },
            None => Vec::new(),
        };
        Ok(Self {
            path,
            lines: if lines.is_empty() {
                vec![String::new()]
            } else {
                lines
            },
            cursor: (0, 0),
            scroll: 0,
            quit: false,
        })
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.quit {
            terminal.draw(|frame| self.draw(frame))?;
            if let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                self.handle_key(key);
            }
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let [text_area, status_area] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(frame.area());

        // Keep the cursor line on screen.
        let height = text_area.height as usize;
        if self.cursor.0 < self.scroll {
            self.scroll = self.cursor.0;
        } else if height > 0 && self.cursor.0 >= self.scroll + height {
            self.scroll = self.cursor.0 + 1 - height;
        }

        let visible: Vec<Line> = self
            .lines
            .iter()
            .skip(self.scroll)
            .take(height)
            .map(|l| Line::raw(l.as_str()))
            .collect();
        frame.render_widget(Paragraph::new(visible), text_area);

        let name = self
            .path
            .as_ref()
            .map_or("[no name]".into(), |p| p.display().to_string());
        let status = format!(
            " {name}  {}:{}   Ctrl+Q quit",
            self.cursor.0 + 1,
            self.cursor.1 + 1
        );
        frame.render_widget(
            Paragraph::new(status).style(Style::new().reversed()),
            status_area,
        );

        frame.set_cursor_position(Position::new(
            text_area.x + self.cursor.1 as u16,
            text_area.y + (self.cursor.0 - self.scroll) as u16,
        ));
    }

    fn handle_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Char('q') if ctrl => self.quit = true,
            KeyCode::Up => self.move_to(self.cursor.0.saturating_sub(1)),
            KeyCode::Down => self.move_to(self.cursor.0 + 1),
            KeyCode::Left => self.cursor.1 = self.cursor.1.saturating_sub(1),
            KeyCode::Right => self.cursor.1 = (self.cursor.1 + 1).min(self.line_len()),
            KeyCode::Home => self.cursor.1 = 0,
            KeyCode::End => self.cursor.1 = self.line_len(),
            _ => {}
        }
    }

    /// Moves to `line` (clamped), keeping the column inside the new line.
    fn move_to(&mut self, line: usize) {
        self.cursor.0 = line.min(self.lines.len() - 1);
        self.cursor.1 = self.cursor.1.min(self.line_len());
    }

    fn line_len(&self) -> usize {
        self.lines[self.cursor.0].chars().count()
    }
}

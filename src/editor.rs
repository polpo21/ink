use std::{env, fs, io, path::PathBuf};

use anyhow::{Context, Result};
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{
        self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
        MouseEventKind,
    },
    layout::{Constraint, Layout, Position, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::{
    buffer::{Buffer, Pos, byte_idx},
    clipboard::Clipboard,
    config::Config,
    help,
    highlight::{self, Highlighter, Kind},
    history::History,
};

const SCROLL_HELP: usize = 3;
const CONFIRM: &str = "Replace?  y yes · n skip · a all · Esc stop";

#[derive(Clone, Copy)]
enum PromptKind {
    SaveAs,
    Find,
    GoTo,
    Replace,
    ReplaceWith,
}

impl PromptKind {
    fn label(self) -> &'static str {
        match self {
            Self::SaveAs => "Save as: ",
            Self::Find => "Find: ",
            Self::GoTo => "Go to line: ",
            Self::Replace => "Replace: ",
            Self::ReplaceWith => "Replace with: ",
        }
    }
}

pub struct Editor {
    path: Option<PathBuf>,
    buf: Buffer,
    history: History,
    highlighter: Option<Highlighter>,
    config: Config,
    /// Indentation settings for the current language.
    tab_width: usize,
    tabs_to_spaces: bool,
    clipboard: Clipboard,
    /// Set when a whole line was copied with nothing selected: pasting it
    /// inserts it above the cursor line instead of at the cursor.
    line_clip: Option<String>,
    cursor: Pos,
    /// The other end of the selection, if any.
    anchor: Option<Pos>,
    /// Display column to aim for when moving up and down.
    goal: Option<usize>,
    /// First visible line and display column.
    scroll: usize,
    hscroll: usize,
    /// Whether the view should scroll to keep the cursor visible.
    follow: bool,
    /// Where the text was drawn last, for mouse clicks and page moves.
    text_area: Rect,
    prompt: Option<(PromptKind, String)>,
    search: String,
    replacement: String,
    /// Asking whether to replace the selected match.
    confirm: bool,
    /// The shortcuts panel is open, scrolled by this many lines.
    help: Option<usize>,
    message: Option<String>,
    quit_armed: bool,
    quit: bool,
}

impl Editor {
    pub fn open(path: Option<PathBuf>, config: Config, message: Option<String>) -> Result<Self> {
        let buf = match &path {
            Some(p) => match fs::read_to_string(p) {
                Ok(text) => Buffer::from_text(&text),
                Err(e) if e.kind() == io::ErrorKind::NotFound => Buffer::new(),
                Err(e) => return Err(e).with_context(|| format!("cannot open {}", p.display())),
            },
            None => Buffer::new(),
        };
        let highlighter = detect(path.as_ref(), &buf);
        let (tab_width, tabs_to_spaces) = config.indent_for(highlighter.as_ref().map(|h| h.name()));
        Ok(Self {
            path,
            buf,
            history: History::new(),
            highlighter,
            config,
            tab_width,
            tabs_to_spaces,
            clipboard: Clipboard::default(),
            line_clip: None,
            cursor: Pos::default(),
            anchor: None,
            goal: None,
            scroll: 0,
            hscroll: 0,
            follow: true,
            text_area: Rect::default(),
            prompt: None,
            search: String::new(),
            replacement: String::new(),
            confirm: false,
            help: None,
            message,
            quit_armed: false,
            quit: false,
        })
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.quit {
            terminal.draw(|frame| self.draw(frame))?;
            match event::read()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => self.on_key(key),
                Event::Paste(text) => self.insert_text(&normalize(&text), false),
                Event::Mouse(mouse) => self.on_mouse(mouse),
                _ => {}
            }
        }
        Ok(())
    }

    // ---- Input ----

    fn on_key(&mut self, key: KeyEvent) {
        if self.help.is_some() {
            return self.on_help_key(key);
        }
        if self.prompt.is_some() {
            return self.on_prompt_key(key);
        }
        if self.confirm {
            return self.on_confirm_key(key);
        }
        self.message = None;
        self.follow = true;
        let quit_armed = std::mem::take(&mut self.quit_armed);
        let m = key.modifiers;
        // Ctrl+Alt is AltGr on many layouts: treat it as plain typing.
        let ctrl = m.contains(KeyModifiers::CONTROL) && !m.contains(KeyModifiers::ALT);
        let shift = m.contains(KeyModifiers::SHIFT);

        if ctrl && let KeyCode::Char(c) = key.code {
            match c.to_ascii_lowercase() {
                'q' => self.quit(quit_armed),
                's' => self.save(),
                'c' => self.copy(),
                'x' => self.cut(),
                'v' => self.paste(),
                'z' if shift => self.redo(),
                'z' => self.undo(),
                'y' => self.redo(),
                'a' => self.select_all(),
                'f' => self.open_prompt(PromptKind::Find),
                'h' => self.open_prompt(PromptKind::Replace),
                'g' => self.open_prompt(PromptKind::GoTo),
                _ => {}
            }
            return;
        }
        match key.code {
            KeyCode::Left
            | KeyCode::Right
            | KeyCode::Up
            | KeyCode::Down
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::PageUp
            | KeyCode::PageDown => self.on_move(key.code, ctrl, shift),
            KeyCode::Char(_)
                if m.contains(KeyModifiers::ALT) && !m.contains(KeyModifiers::CONTROL) => {}
            KeyCode::Char(c) => self.insert_text(c.encode_utf8(&mut [0; 4]), true),
            KeyCode::Enter => self.newline(),
            KeyCode::Tab => self.insert_tab(),
            KeyCode::Backspace if ctrl => self.delete_word(false),
            KeyCode::Delete if ctrl => self.delete_word(true),
            KeyCode::Backspace => self.backspace(),
            KeyCode::Delete => self.delete_forward(),
            KeyCode::Esc => self.anchor = None,
            KeyCode::F(1) => self.help = Some(0),
            KeyCode::F(3) => {
                self.find_next();
            }
            _ => {}
        }
    }

    fn on_prompt_key(&mut self, key: KeyEvent) {
        let Some((kind, input)) = &mut self.prompt else {
            return;
        };
        match key.code {
            KeyCode::Esc => self.prompt = None,
            KeyCode::Enter => {
                let (kind, input) = (*kind, std::mem::take(input));
                self.prompt = None;
                match kind {
                    PromptKind::SaveAs if !input.is_empty() => {
                        self.path = Some(expand_home(&input));
                        self.highlighter = detect(self.path.as_ref(), &self.buf);
                        (self.tab_width, self.tabs_to_spaces) = self
                            .config
                            .indent_for(self.highlighter.as_ref().map(|h| h.name()));
                        self.save();
                    }
                    PromptKind::Find if !input.is_empty() => {
                        self.search = input;
                        self.find_next();
                    }
                    PromptKind::GoTo => self.go_to(&input),
                    PromptKind::Replace if !input.is_empty() => {
                        self.search = input;
                        self.open_prompt(PromptKind::ReplaceWith);
                    }
                    PromptKind::ReplaceWith => {
                        self.replacement = input;
                        self.confirm = self.find_next();
                    }
                    PromptKind::SaveAs | PromptKind::Find | PromptKind::Replace => {}
                }
            }
            KeyCode::Backspace => {
                input.pop();
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => input.push(c),
            _ => {}
        }
    }

    fn on_help_key(&mut self, key: KeyEvent) {
        let Some(scroll) = &mut self.help else {
            return;
        };
        match key.code {
            KeyCode::Up => *scroll = scroll.saturating_sub(1),
            KeyCode::Down => *scroll += 1,
            KeyCode::PageUp => *scroll = scroll.saturating_sub(10),
            KeyCode::PageDown => *scroll += 10,
            KeyCode::Home => *scroll = 0,
            KeyCode::End => *scroll = usize::MAX,
            _ => self.help = None,
        }
    }

    fn on_confirm_key(&mut self, key: KeyEvent) {
        self.message = None;
        match key.code {
            KeyCode::Char('y' | 'Y') | KeyCode::Enter => {
                self.replace_selection();
                self.confirm = self.find_next();
            }
            KeyCode::Char('n' | 'N') => self.confirm = self.find_next(),
            KeyCode::Char('a' | 'A') => {
                self.confirm = false;
                self.replace_all();
            }
            KeyCode::Esc | KeyCode::Char('q' | 'Q') => {
                self.confirm = false;
                self.anchor = None;
            }
            _ => {}
        }
    }

    fn on_mouse(&mut self, mouse: MouseEvent) {
        if let Some(scroll) = &mut self.help {
            match mouse.kind {
                MouseEventKind::ScrollUp => *scroll = scroll.saturating_sub(SCROLL_HELP),
                MouseEventKind::ScrollDown => *scroll += SCROLL_HELP,
                MouseEventKind::Down(_) => self.help = None,
                _ => {}
            }
            return;
        }
        let area = self.text_area;
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if mouse.row < area.y || mouse.row >= area.bottom() {
                    return;
                }
                let p = self.pos_at(mouse.column, mouse.row);
                self.cursor = p;
                self.anchor = Some(p);
                self.goal = None;
                self.follow = true;
                self.message = None;
                self.quit_armed = false;
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                self.cursor = self.pos_at(mouse.column, mouse.row);
                self.goal = None;
                self.follow = true;
            }
            MouseEventKind::ScrollUp => {
                self.scroll = self.scroll.saturating_sub(self.config.scroll_lines);
                self.follow = false;
            }
            MouseEventKind::ScrollDown => {
                self.scroll =
                    (self.scroll + self.config.scroll_lines).min(self.buf.len_lines() - 1);
                self.follow = false;
            }
            _ => {}
        }
    }

    // ---- Movement ----

    fn on_move(&mut self, code: KeyCode, ctrl: bool, shift: bool) {
        if shift {
            self.anchor.get_or_insert(self.cursor);
        } else if let Some((start, end)) = self.selection() {
            // Left/Right with a selection jump to its edge, like everywhere else.
            self.anchor = None;
            match code {
                KeyCode::Left => return self.set_cursor(start),
                KeyCode::Right => return self.set_cursor(end),
                _ => {}
            }
        } else {
            self.anchor = None;
        }

        let Pos { line, col } = self.cursor;
        let last = self.buf.len_lines() - 1;
        let page = (self.text_area.height as usize).max(1);
        match code {
            KeyCode::Up if line == 0 => self.set_cursor(Pos::new(0, 0)),
            KeyCode::Up => self.move_vertically(line - 1),
            KeyCode::Down if line == last => self.set_cursor(self.buf.end()),
            KeyCode::Down => self.move_vertically(line + 1),
            KeyCode::PageUp => {
                self.scroll = self.scroll.saturating_sub(page);
                self.move_vertically(line.saturating_sub(page));
            }
            KeyCode::PageDown => {
                self.scroll = (self.scroll + page).min(last);
                self.move_vertically((line + page).min(last));
            }
            KeyCode::Left if ctrl => self.set_cursor(self.word_left()),
            KeyCode::Right if ctrl => self.set_cursor(self.word_right()),
            KeyCode::Left if col > 0 => self.set_cursor(Pos::new(line, col - 1)),
            KeyCode::Left if line > 0 => {
                self.set_cursor(Pos::new(line - 1, self.buf.line_len(line - 1)))
            }
            KeyCode::Right if col < self.buf.line_len(line) => {
                self.set_cursor(Pos::new(line, col + 1))
            }
            KeyCode::Right if line < last => self.set_cursor(Pos::new(line + 1, 0)),
            KeyCode::Home if ctrl => self.set_cursor(Pos::new(0, 0)),
            KeyCode::End if ctrl => self.set_cursor(self.buf.end()),
            KeyCode::Home => {
                // First press goes to the indentation, second to column 0.
                let indent = self
                    .buf
                    .line(line)
                    .chars()
                    .take_while(|c| c.is_whitespace())
                    .count();
                self.set_cursor(Pos::new(line, if col == indent { 0 } else { indent }));
            }
            KeyCode::End => self.set_cursor(Pos::new(line, self.buf.line_len(line))),
            _ => {}
        }
    }

    fn set_cursor(&mut self, pos: Pos) {
        self.cursor = pos;
        self.goal = None;
    }

    fn move_vertically(&mut self, line: usize) {
        let goal = self.goal.unwrap_or_else(|| self.display_col(self.cursor));
        self.cursor = Pos::new(line, self.col_at(line, goal));
        self.goal = Some(goal);
    }

    fn word_left(&self) -> Pos {
        let Pos { line, col } = self.cursor;
        if col == 0 {
            return if line > 0 {
                Pos::new(line - 1, self.buf.line_len(line - 1))
            } else {
                self.cursor
            };
        }
        let chars: Vec<char> = self.buf.line(line).chars().take(col).collect();
        let mut i = chars.len();
        while i > 0 && chars[i - 1].is_whitespace() {
            i -= 1;
        }
        if i > 0 {
            let word = is_word(chars[i - 1]);
            while i > 0 && !chars[i - 1].is_whitespace() && is_word(chars[i - 1]) == word {
                i -= 1;
            }
        }
        Pos::new(line, i)
    }

    fn word_right(&self) -> Pos {
        let Pos { line, col } = self.cursor;
        let chars: Vec<char> = self.buf.line(line).chars().collect();
        if col >= chars.len() {
            return if line + 1 < self.buf.len_lines() {
                Pos::new(line + 1, 0)
            } else {
                self.cursor
            };
        }
        let mut i = col;
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i < chars.len() {
            let word = is_word(chars[i]);
            while i < chars.len() && !chars[i].is_whitespace() && is_word(chars[i]) == word {
                i += 1;
            }
        }
        Pos::new(line, i)
    }

    fn selection(&self) -> Option<(Pos, Pos)> {
        let anchor = self.anchor.filter(|a| *a != self.cursor)?;
        Some((anchor.min(self.cursor), anchor.max(self.cursor)))
    }

    fn select_all(&mut self) {
        self.anchor = Some(Pos::new(0, 0));
        self.set_cursor(self.buf.end());
    }

    // ---- Editing ----

    fn insert_text(&mut self, text: &str, merge: bool) {
        // Replacing a selection is one undo step.
        let grouped = self.selection().is_some();
        if grouped {
            self.history.begin_group();
            self.delete_selection();
        }
        self.anchor = None;
        let at = self.cursor;
        let end = self.history.insert(&mut self.buf, at, text, at, merge);
        self.set_cursor(end);
        if grouped {
            self.history.end_group();
        }
    }

    fn delete_range(&mut self, start: Pos, end: Pos, merge: bool) {
        self.history
            .delete(&mut self.buf, start, end, self.cursor, merge);
        self.set_cursor(start);
    }

    fn delete_selection(&mut self) -> bool {
        let Some((start, end)) = self.selection() else {
            self.anchor = None;
            return false;
        };
        self.anchor = None;
        self.delete_range(start, end, false);
        true
    }

    fn newline(&mut self) {
        // Keep the current line's indentation.
        let indent: String = if self.config.auto_indent {
            self.buf
                .line(self.cursor.line)
                .chars()
                .take(self.cursor.col)
                .take_while(|c| c.is_whitespace())
                .collect()
        } else {
            String::new()
        };
        self.insert_text(&format!("\n{indent}"), false);
    }

    fn insert_tab(&mut self) {
        if self.tabs_to_spaces {
            // Up to the next tab stop.
            let n = self.tab_width - self.display_col(self.cursor) % self.tab_width;
            self.insert_text(&" ".repeat(n), false);
        } else {
            self.insert_text("\t", false);
        }
    }

    fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }
        let Pos { line, col } = self.cursor;
        let start = if col > 0 {
            Pos::new(line, col - 1)
        } else if line > 0 {
            Pos::new(line - 1, self.buf.line_len(line - 1))
        } else {
            return;
        };
        self.delete_range(start, self.cursor, true);
    }

    fn delete_forward(&mut self) {
        if self.delete_selection() {
            return;
        }
        let Pos { line, col } = self.cursor;
        let end = if col < self.buf.line_len(line) {
            Pos::new(line, col + 1)
        } else if line + 1 < self.buf.len_lines() {
            Pos::new(line + 1, 0)
        } else {
            return;
        };
        self.delete_range(self.cursor, end, true);
    }

    /// Ctrl+Backspace / Ctrl+Delete: delete the word before or after the cursor.
    fn delete_word(&mut self, forward: bool) {
        if self.delete_selection() {
            return;
        }
        let (start, end) = if forward {
            (self.cursor, self.word_right())
        } else {
            (self.word_left(), self.cursor)
        };
        if start != end {
            self.delete_range(start, end, false);
        }
    }

    fn copy(&mut self) {
        if let Some((start, end)) = self.selection() {
            self.clipboard.copy(&self.buf.text(start, end));
            self.line_clip = None;
        } else {
            let text = format!("{}\n", self.buf.line(self.cursor.line));
            self.clipboard.copy(&text);
            self.line_clip = Some(text);
        }
        self.message = Some("Copied".into());
    }

    fn cut(&mut self) {
        self.copy();
        if self.delete_selection() {
            self.message = Some("Cut".into());
            return;
        }
        // Nothing selected: cut the whole line.
        let Pos { line, col } = self.cursor;
        let len = self.buf.line_len(line);
        let (start, end) = if line + 1 < self.buf.len_lines() {
            (Pos::new(line, 0), Pos::new(line + 1, 0))
        } else if line > 0 {
            (
                Pos::new(line - 1, self.buf.line_len(line - 1)),
                Pos::new(line, len),
            )
        } else {
            (Pos::new(0, 0), Pos::new(0, len))
        };
        self.delete_range(start, end, false);
        let line = self.cursor.line.min(line);
        self.set_cursor(Pos::new(line, col.min(self.buf.line_len(line))));
        self.message = Some("Cut line".into());
    }

    fn paste(&mut self) {
        let text = normalize(&self.clipboard.paste());
        if text.is_empty() {
            return;
        }
        if self.selection().is_none() && self.line_clip.as_deref() == Some(text.as_str()) {
            let Pos { line, col } = self.cursor;
            self.history
                .insert(&mut self.buf, Pos::new(line, 0), &text, self.cursor, false);
            self.set_cursor(Pos::new(line + 1, col));
        } else {
            self.insert_text(&text, false);
        }
    }

    fn undo(&mut self) {
        match self.history.undo(&mut self.buf) {
            Some(pos) => self.set_cursor(pos),
            None => self.message = Some("Nothing to undo".into()),
        }
        self.anchor = None;
    }

    fn redo(&mut self) {
        match self.history.redo(&mut self.buf) {
            Some(pos) => self.set_cursor(pos),
            None => self.message = Some("Nothing to redo".into()),
        }
        self.anchor = None;
    }

    // ---- Files, search, quit ----

    fn save(&mut self) {
        let Some(path) = &self.path else {
            return self.open_prompt(PromptKind::SaveAs);
        };
        // Written in place, so symlinked files (e.g. dotfiles) stay symlinks.
        self.message = Some(match fs::write(path, self.buf.to_text()) {
            Ok(()) => {
                self.history.mark_saved();
                format!("Saved {} ({} lines)", path.display(), self.buf.len_lines())
            }
            Err(e) => format!("Save failed: {e}"),
        });
    }

    fn quit(&mut self, armed: bool) {
        if self.history.is_dirty() && !armed {
            self.message = Some("Unsaved changes! Ctrl+S to save, Ctrl+Q again to quit".into());
            self.quit_armed = true;
        } else {
            self.quit = true;
        }
    }

    fn open_prompt(&mut self, kind: PromptKind) {
        let input = match kind {
            PromptKind::Find | PromptKind::Replace => self.search.clone(),
            PromptKind::ReplaceWith => self.replacement.clone(),
            PromptKind::SaveAs | PromptKind::GoTo => String::new(),
        };
        self.prompt = Some((kind, input));
    }

    /// Ctrl+G: `42` goes to line 42, `42:7` to column 7 of it.
    fn go_to(&mut self, input: &str) {
        let mut parts = input.trim().splitn(2, ':');
        let Some(line) = parts.next().and_then(|l| l.trim().parse::<usize>().ok()) else {
            self.message = Some(format!("Not a line number: {input}"));
            return;
        };
        let line = line.clamp(1, self.buf.len_lines()) - 1;
        let col = parts
            .next()
            .and_then(|c| c.trim().parse::<usize>().ok())
            .unwrap_or(1);
        self.anchor = None;
        self.set_cursor(Pos::new(
            line,
            (col.max(1) - 1).min(self.buf.line_len(line)),
        ));
        // Show the line in the middle of the screen.
        self.scroll = line.saturating_sub(self.text_area.height as usize / 2);
        self.follow = true;
    }

    /// The search as it should be matched: searches are case-insensitive
    /// (and the needle lowercased) unless they contain an uppercase letter.
    fn needle(&self) -> (String, bool) {
        let insensitive = !self.search.chars().any(char::is_uppercase);
        let needle = if insensitive {
            self.search.to_lowercase()
        } else {
            self.search.clone()
        };
        (needle, insensitive)
    }

    /// Selects the next match after the cursor, wrapping around the end.
    fn find_next(&mut self) -> bool {
        if self.search.is_empty() {
            return false;
        }
        let (needle, insensitive) = self.needle();
        let from = self.selection().map_or(self.cursor, |(_, end)| end);
        let n = self.buf.len_lines();
        for i in 0..=n {
            let li = (from.line + i) % n;
            let line = self.buf.line(li);
            let start = if i == 0 { byte_idx(line, from.col) } else { 0 };
            if let Some((a, b)) = find_in(&line[start..], &needle, insensitive) {
                let col = line[..start + a].chars().count();
                let end = line[..start + b].chars().count();
                self.anchor = Some(Pos::new(li, col));
                self.set_cursor(Pos::new(li, end));
                self.follow = true;
                if li < from.line || i == n {
                    self.message = Some("Search wrapped to the top".into());
                }
                return true;
            }
        }
        self.anchor = None;
        self.message = Some(format!("Not found: {}", self.search));
        false
    }

    fn replace_selection(&mut self) {
        let Some((start, end)) = self.selection() else {
            return;
        };
        self.history.begin_group();
        self.delete_range(start, end, false);
        if !self.replacement.is_empty() {
            let end = self
                .history
                .insert(&mut self.buf, start, &self.replacement, start, false);
            self.set_cursor(end);
        }
        self.history.end_group();
        self.history.seal();
        self.anchor = None;
    }

    /// Replaces every match in the file, as a single undo step.
    fn replace_all(&mut self) {
        let (needle, insensitive) = self.needle();
        let mut matches = Vec::new();
        for li in 0..self.buf.len_lines() {
            let line = self.buf.line(li);
            let mut from = 0;
            while let Some((a, b)) = find_in(&line[from..], &needle, insensitive) {
                let start = Pos::new(li, line[..from + a].chars().count());
                let end = Pos::new(li, line[..from + b].chars().count());
                matches.push((start, end));
                from += b;
            }
        }
        if matches.is_empty() {
            self.message = Some(format!("Not found: {}", self.search));
            return;
        }
        let cursor = self.cursor;
        self.history.begin_group();
        // Back to front, so earlier positions stay valid.
        for &(start, end) in matches.iter().rev() {
            self.history
                .delete(&mut self.buf, start, end, cursor, false);
            if !self.replacement.is_empty() {
                self.history
                    .insert(&mut self.buf, start, &self.replacement, cursor, false);
            }
        }
        self.history.end_group();
        self.history.seal();
        self.anchor = None;
        let line = cursor.line;
        self.set_cursor(Pos::new(line, cursor.col.min(self.buf.line_len(line))));
        let s = if matches.len() == 1 { "" } else { "s" };
        self.message = Some(format!("Replaced {} occurrence{s}", matches.len()));
    }

    // ---- Drawing ----

    fn draw(&mut self, frame: &mut Frame) {
        let [main, status_area] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(frame.area());
        let n = self.buf.len_lines();
        let gutter = if self.config.line_numbers {
            n.to_string().len().max(3) + 1
        } else {
            0
        };
        let [gutter_area, text_area] =
            Layout::horizontal([Constraint::Length(gutter as u16), Constraint::Min(1)]).areas(main);
        self.text_area = text_area;
        let (height, width) = (text_area.height as usize, text_area.width as usize);

        let cursor_col = self.display_col(self.cursor);
        if self.follow {
            if self.cursor.line < self.scroll {
                self.scroll = self.cursor.line;
            } else if self.cursor.line >= self.scroll + height {
                self.scroll = self.cursor.line + 1 - height;
            }
            if cursor_col < self.hscroll {
                self.hscroll = cursor_col;
            } else if cursor_col >= self.hscroll + width {
                self.hscroll = cursor_col + 1 - width;
            }
        }

        if let Some(line) = self.buf.take_changed_from()
            && let Some(h) = &mut self.highlighter
        {
            h.invalidate(line);
        }
        let visible = self.scroll..(self.scroll + height).min(n);
        let numbers: Vec<Line> = visible
            .clone()
            .map(|i| {
                let colors = &self.config.colors;
                let color = if i == self.cursor.line {
                    colors.current_line_number
                } else {
                    colors.line_number
                };
                Line::styled(
                    format!("{:>w$} ", i + 1, w = gutter.saturating_sub(1)),
                    Style::new().fg(color),
                )
            })
            .collect();
        frame.render_widget(Paragraph::new(numbers), gutter_area);

        let selection = self.selection();
        let lines: Vec<Line> = visible
            .map(|i| {
                let kinds = match &mut self.highlighter {
                    Some(h) => h.line(&self.buf, i),
                    None => Vec::new(),
                };
                self.render_line(i, &kinds, selection, width)
            })
            .collect();
        frame.render_widget(Paragraph::new(lines), text_area);

        self.draw_status(frame, status_area);

        if let Some(scroll) = &mut self.help {
            let c = &self.config.colors;
            help::draw(frame, frame.area(), scroll, c.keyword, c.line_number);
        } else if let Some((kind, input)) = &self.prompt {
            let x = 1 + kind.label().width() + input.width();
            frame.set_cursor_position(Position::new(status_area.x + x as u16, status_area.y));
        } else if (self.scroll..self.scroll + height).contains(&self.cursor.line)
            && (self.hscroll..self.hscroll + width).contains(&cursor_col)
        {
            frame.set_cursor_position(Position::new(
                text_area.x + (cursor_col - self.hscroll) as u16,
                text_area.y + (self.cursor.line - self.scroll) as u16,
            ));
        }
    }

    fn render_line(
        &self,
        i: usize,
        kinds: &[Kind],
        selection: Option<(Pos, Pos)>,
        width: usize,
    ) -> Line<'static> {
        let selected_style = Style::new().add_modifier(Modifier::REVERSED);
        let is_selected = |col| selection.is_some_and(|(s, e)| (s..e).contains(&Pos::new(i, col)));
        let (start, stop) = (self.hscroll, self.hscroll + width);

        let mut spans = Vec::new();
        let mut run = String::new();
        let mut run_style = Style::new();
        let mut dcol = 0;
        for (col, ch) in self.buf.line(i).chars().enumerate() {
            let cells = char_width(ch, dcol, self.tab_width);
            if dcol + cells <= start {
                dcol += cells;
                continue;
            }
            if dcol >= stop {
                break;
            }
            let mut style = self.kind_style(kinds.get(col).copied().unwrap_or(Kind::Text));
            if is_selected(col) {
                style = style.patch(selected_style);
            }
            if style != run_style && !run.is_empty() {
                spans.push(Span::styled(std::mem::take(&mut run), run_style));
            }
            run_style = style;
            if ch == '\t' || dcol < start || dcol + cells > stop {
                // Tabs, and wide characters cut by the edge, become spaces.
                let visible = (dcol + cells).min(stop) - dcol.max(start);
                run.extend(std::iter::repeat_n(' ', visible));
            } else if ch.is_control() {
                run.push('?');
            } else {
                run.push(ch);
            }
            dcol += cells;
        }
        if !run.is_empty() {
            spans.push(Span::styled(run, run_style));
        }
        // Show the selected line break as a highlighted space.
        if is_selected(self.buf.line_len(i)) && (start..stop).contains(&dcol) {
            spans.push(Span::styled(" ", selected_style));
        }
        Line::from(spans)
    }

    fn draw_status(&self, frame: &mut Frame, area: Rect) {
        let c = &self.config.colors;
        let accent = Style::new().fg(c.keyword).add_modifier(Modifier::BOLD);
        let dim = Style::new().fg(c.line_number);
        let (left, right) = if let Some((kind, input)) = &self.prompt {
            let left = vec![
                Span::raw(" "),
                Span::styled(kind.label(), accent),
                Span::raw(input.clone()),
            ];
            (left, "Enter ok · Esc cancel ".to_owned())
        } else {
            let name = self
                .path
                .as_ref()
                .map_or("new file".into(), |p| p.display().to_string());
            let mut left = vec![
                Span::raw(" "),
                Span::styled(name, Style::new().add_modifier(Modifier::BOLD)),
            ];
            if self.history.is_dirty() {
                left.push(Span::styled(" ●", Style::new().fg(c.current_line_number)));
            }
            if let Some(message) = &self.message {
                left.push(Span::raw(format!("   {message}")));
            } else if self.confirm {
                left.push(Span::styled(format!("   {CONFIRM}"), accent));
            }
            let lang = self
                .highlighter
                .as_ref()
                .map_or(String::new(), |h| format!("{}  ", h.name()));
            let right = format!(
                "{lang}{}:{}   F1 help ",
                self.cursor.line + 1,
                self.cursor.col + 1
            );
            (left, right)
        };
        let used: usize = left.iter().map(|s| s.content.width()).sum();
        let pad = (area.width as usize).saturating_sub(used + right.width());
        let mut spans = left;
        spans.push(Span::raw(" ".repeat(pad)));
        spans.push(Span::styled(right, dim));
        frame.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    fn kind_style(&self, kind: Kind) -> Style {
        let c = &self.config.colors;
        let style = Style::new();
        match kind {
            Kind::Text => style,
            Kind::Comment => style.fg(c.comment).add_modifier(Modifier::ITALIC),
            Kind::String => style.fg(c.string),
            Kind::Number => style.fg(c.number),
            Kind::Constant => style.fg(c.constant),
            Kind::Keyword => style.fg(c.keyword),
            Kind::Type => style.fg(c.type_),
            Kind::Function => style.fg(c.function),
            Kind::Heading => style.fg(c.heading).add_modifier(Modifier::BOLD),
        }
    }

    // ---- Columns ----

    /// The screen column (tabs and wide characters expanded) of `pos`.
    fn display_col(&self, pos: Pos) -> usize {
        self.buf
            .line(pos.line)
            .chars()
            .take(pos.col)
            .fold(0, |dcol, ch| dcol + char_width(ch, dcol, self.tab_width))
    }

    /// The character column on `line` closest to screen column `target`.
    fn col_at(&self, line: usize, target: usize) -> usize {
        let mut dcol = 0;
        for (col, ch) in self.buf.line(line).chars().enumerate() {
            let cells = char_width(ch, dcol, self.tab_width);
            if dcol + cells > target {
                return col;
            }
            dcol += cells;
        }
        self.buf.line_len(line)
    }

    /// The buffer position under a screen cell, clamped to the text.
    fn pos_at(&self, x: u16, y: u16) -> Pos {
        let area = self.text_area;
        let row = y.clamp(area.y, area.bottom().saturating_sub(1)) - area.y;
        let line = (self.scroll + row as usize).min(self.buf.len_lines() - 1);
        let dcol = self.hscroll + x.saturating_sub(area.x) as usize;
        Pos::new(line, self.col_at(line, dcol))
    }
}

fn char_width(ch: char, dcol: usize, tab_width: usize) -> usize {
    match ch {
        '\t' => tab_width - dcol % tab_width,
        c if c.is_control() => 1,
        c => c.width().unwrap_or(0),
    }
}

fn detect(path: Option<&PathBuf>, buf: &Buffer) -> Option<Highlighter> {
    highlight::detect(path?, buf.line(0)).map(Highlighter::new)
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Unifies pasted line endings to `\n`.
fn normalize(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Finds `needle` in `hay` and returns its byte range in `hay`.
/// When `insensitive`, `needle` must already be lowercase.
fn find_in(hay: &str, needle: &str, insensitive: bool) -> Option<(usize, usize)> {
    if !insensitive {
        return hay.find(needle).map(|a| (a, a + needle.len()));
    }
    if hay.is_ascii() {
        return hay
            .to_ascii_lowercase()
            .find(needle)
            .map(|a| (a, a + needle.len()));
    }
    // Lowercasing can change byte lengths outside ASCII: compare char by char.
    hay.char_indices().find_map(|(a, _)| {
        let mut rest = hay[a..]
            .char_indices()
            .flat_map(|(i, c)| c.to_lowercase().map(move |l| (a + i + c.len_utf8(), l)));
        let mut end = a;
        for n in needle.chars() {
            let (e, l) = rest.next()?;
            if l != n {
                return None;
            }
            end = e;
        }
        Some((a, end))
    })
}

fn expand_home(path: &str) -> PathBuf {
    match (path.strip_prefix("~/"), env::var_os("HOME")) {
        (Some(rest), Some(home)) => PathBuf::from(home).join(rest),
        _ => PathBuf::from(path),
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    fn screen(editor: &mut Editor, terminal: &mut Terminal<TestBackend>) -> String {
        terminal.draw(|frame| editor.draw(frame)).unwrap();
        let buf = terminal.backend().buffer();
        buf.content
            .chunks(buf.area.width as usize)
            .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn draws_status_bar_and_help_panel() {
        let mut editor = Editor::open(None, Config::default(), None).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(72, 24)).unwrap();
        editor.insert_text("fn main() {}", false);
        let text = screen(&mut editor, &mut terminal);
        println!("{text}");
        assert!(text.contains("new file ●"));
        assert!(text.contains("1:13   F1 help"));

        editor.on_key(KeyEvent::from(KeyCode::F(1)));
        let text = screen(&mut editor, &mut terminal);
        println!("{text}");
        assert!(text.contains("Shortcuts"));
        assert!(text.contains("Ctrl+S"));

        editor.on_key(KeyEvent::from(KeyCode::Esc));
        assert!(!screen(&mut editor, &mut terminal).contains("Shortcuts"));
    }

    #[test]
    fn finds_case_insensitively() {
        assert_eq!(find_in("Hello World", "world", true), Some((6, 11)));
        assert_eq!(find_in("Hello World", "world", false), None);
        assert_eq!(find_in("Caffè ÈCCO", "èc", true), Some((7, 10)));
    }
}

//! The text buffer: a list of lines plus the file's line-ending style.

/// A position in the buffer. `col` counts characters, not bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pos {
    pub line: usize,
    pub col: usize,
}

impl Pos {
    pub const fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

pub struct Buffer {
    lines: Vec<String>,
    trailing_newline: bool,
    crlf: bool,
    /// First line changed since the last `take_changed_from`.
    changed_from: Option<usize>,
}

impl Buffer {
    /// An empty buffer for a new file.
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            trailing_newline: true,
            crlf: false,
            changed_from: None,
        }
    }

    pub fn from_text(text: &str) -> Self {
        let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
        if lines.is_empty() {
            lines.push(String::new());
        }
        Self {
            lines,
            trailing_newline: text.ends_with('\n'),
            crlf: text.contains("\r\n"),
            changed_from: None,
        }
    }

    /// The contents as they should be written to disk, with the original line endings.
    pub fn to_text(&self) -> String {
        let eol = if self.crlf { "\r\n" } else { "\n" };
        let mut text = self.lines.join(eol);
        if self.trailing_newline {
            text.push_str(eol);
        }
        text
    }

    pub fn len_lines(&self) -> usize {
        self.lines.len()
    }

    pub fn line(&self, i: usize) -> &str {
        &self.lines[i]
    }

    pub fn line_len(&self, i: usize) -> usize {
        self.lines[i].chars().count()
    }

    pub fn end(&self) -> Pos {
        let last = self.lines.len() - 1;
        Pos::new(last, self.line_len(last))
    }

    /// The text between `start` and `end` (`start <= end`), lines joined by `\n`.
    pub fn text(&self, start: Pos, end: Pos) -> String {
        let first = &self.lines[start.line];
        let a = byte_idx(first, start.col);
        if start.line == end.line {
            return first[a..byte_idx(first, end.col)].to_owned();
        }
        let mut text = first[a..].to_owned();
        for line in &self.lines[start.line + 1..end.line] {
            text.push('\n');
            text.push_str(line);
        }
        let last = &self.lines[end.line];
        text.push('\n');
        text.push_str(&last[..byte_idx(last, end.col)]);
        text
    }

    /// Inserts `text` (which may contain `\n`) at `at` and returns the position after it.
    pub fn insert(&mut self, at: Pos, text: &str) -> Pos {
        self.mark_changed(at.line);
        let line = &mut self.lines[at.line];
        let tail = line.split_off(byte_idx(line, at.col));
        let mut parts = text.split('\n');
        let first = parts.next().unwrap_or_default();
        line.push_str(first);

        let mut new_lines: Vec<String> = parts.map(str::to_owned).collect();
        let end = match new_lines.last() {
            None => Pos::new(at.line, at.col + first.chars().count()),
            Some(last) => Pos::new(at.line + new_lines.len(), last.chars().count()),
        };
        match new_lines.last_mut() {
            None => line.push_str(&tail),
            Some(last) => last.push_str(&tail),
        }
        let next = at.line + 1;
        self.lines.splice(next..next, new_lines);
        end
    }

    /// Removes the text between `start` and `end` (`start <= end`) and returns it.
    pub fn delete(&mut self, start: Pos, end: Pos) -> String {
        self.mark_changed(start.line);
        let removed = self.text(start, end);
        let last = &self.lines[end.line];
        let tail = last[byte_idx(last, end.col)..].to_owned();
        let first = &mut self.lines[start.line];
        first.truncate(byte_idx(first, start.col));
        first.push_str(&tail);
        self.lines.drain(start.line + 1..=end.line);
        removed
    }

    /// Returns (and resets) the first line changed since the last call.
    pub fn take_changed_from(&mut self) -> Option<usize> {
        self.changed_from.take()
    }

    fn mark_changed(&mut self, line: usize) {
        self.changed_from = Some(self.changed_from.map_or(line, |l| l.min(line)));
    }
}

/// Byte offset of the character at `col` in `s` (or the end of `s`).
pub fn byte_idx(s: &str, col: usize) -> usize {
    s.char_indices().nth(col).map_or(s.len(), |(i, _)| i)
}

/// The position reached after inserting `text` at `start`.
pub fn end_pos(start: Pos, text: &str) -> Pos {
    match text.rsplit_once('\n') {
        None => Pos::new(start.line, start.col + text.chars().count()),
        Some((before, after)) => Pos::new(
            start.line + before.matches('\n').count() + 1,
            after.chars().count(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_line_endings() {
        for text in ["", "a", "a\n", "a\n\nb\n", "a\r\nb\r\n"] {
            assert_eq!(Buffer::from_text(text).to_text(), text);
        }
    }

    #[test]
    fn insert_and_delete_are_inverse() {
        let mut buf = Buffer::from_text("hello world\nsecond\n");
        let at = Pos::new(0, 5);
        let end = buf.insert(at, ",\nbig");
        assert_eq!(end, Pos::new(1, 3));
        assert_eq!(end, end_pos(at, ",\nbig"));
        assert_eq!(buf.to_text(), "hello,\nbig world\nsecond\n");
        assert_eq!(buf.delete(at, end), ",\nbig");
        assert_eq!(buf.to_text(), "hello world\nsecond\n");
    }

    #[test]
    fn handles_multibyte_characters() {
        let mut buf = Buffer::from_text("caffè è buono");
        buf.insert(Pos::new(0, 6), "!");
        assert_eq!(buf.line(0), "caffè !è buono");
        assert_eq!(buf.delete(Pos::new(0, 4), Pos::new(0, 7)), "è !");
    }
}

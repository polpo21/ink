//! Undo and redo.

use crate::buffer::{Buffer, Pos, end_pos};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Insert,
    Delete,
}

struct Edit {
    kind: Kind,
    start: Pos,
    text: String,
    /// Where the cursor was before the edit, restored on undo.
    cursor: Pos,
    /// Edits with the same group are undone and redone together.
    group: u64,
}

/// Every change to the buffer goes through here. Consecutive typing and
/// single-character deletions are merged, so one undo removes a whole word.
pub struct History {
    undo: Vec<Edit>,
    redo: Vec<Edit>,
    /// When set, the next edit starts a new undo step.
    sealed: bool,
    /// Length of the undo stack when the file was last saved.
    saved: Option<usize>,
    last_group: u64,
    open_group: Option<u64>,
}

impl History {
    pub fn new() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            sealed: false,
            saved: Some(0),
            last_group: 0,
            open_group: None,
        }
    }

    pub fn insert(
        &mut self,
        buf: &mut Buffer,
        at: Pos,
        text: &str,
        cursor: Pos,
        merge: bool,
    ) -> Pos {
        let end = buf.insert(at, text);
        if merge
            && self.can_merge(Kind::Insert)
            && let Some(last) = self.undo.last_mut()
            // A space after a word starts a new step: undo goes word by word.
            && end_pos(last.start, &last.text) == at
            && !(text.starts_with(char::is_whitespace) && !last.text.ends_with(char::is_whitespace))
        {
            last.text.push_str(text);
            return end;
        }
        self.push(Edit {
            kind: Kind::Insert,
            start: at,
            text: text.to_owned(),
            cursor,
            group: 0,
        });
        end
    }

    pub fn delete(&mut self, buf: &mut Buffer, start: Pos, end: Pos, cursor: Pos, merge: bool) {
        let text = buf.delete(start, end);
        if merge
            && self.can_merge(Kind::Delete)
            && let Some(last) = self.undo.last_mut()
        {
            if end == last.start {
                // Backspace
                last.start = start;
                last.text.insert_str(0, &text);
                return;
            }
            if start == last.start {
                // Delete
                last.text.push_str(&text);
                return;
            }
        }
        self.push(Edit {
            kind: Kind::Delete,
            start,
            text,
            cursor,
            group: 0,
        });
    }

    /// Starts a group: the edits until `end_group` count as one undo step.
    pub fn begin_group(&mut self) {
        self.last_group += 1;
        self.open_group = Some(self.last_group);
        self.sealed = true;
    }

    /// Ends the group. Typing right after still merges into it, so replacing a
    /// selection by typing a word is undone in one step.
    pub fn end_group(&mut self) {
        self.open_group = None;
    }

    /// Makes the next edit start a new undo step.
    pub fn seal(&mut self) {
        self.sealed = true;
    }

    /// Reverts the last undo step and returns where the cursor should go.
    pub fn undo(&mut self, buf: &mut Buffer) -> Option<Pos> {
        let group = self.undo.last()?.group;
        let mut cursor = None;
        while let Some(edit) = self.undo.pop_if(|e| e.group == group) {
            match edit.kind {
                Kind::Insert => {
                    buf.delete(edit.start, end_pos(edit.start, &edit.text));
                }
                Kind::Delete => {
                    buf.insert(edit.start, &edit.text);
                }
            }
            cursor = Some(edit.cursor);
            self.redo.push(edit);
        }
        self.sealed = true;
        cursor
    }

    /// Re-applies the last undone step and returns where the cursor should go.
    pub fn redo(&mut self, buf: &mut Buffer) -> Option<Pos> {
        let group = self.redo.last()?.group;
        let mut cursor = None;
        while let Some(edit) = self.redo.pop_if(|e| e.group == group) {
            cursor = Some(match edit.kind {
                Kind::Insert => buf.insert(edit.start, &edit.text),
                Kind::Delete => {
                    buf.delete(edit.start, end_pos(edit.start, &edit.text));
                    edit.start
                }
            });
            self.undo.push(edit);
        }
        self.sealed = true;
        cursor
    }

    pub fn mark_saved(&mut self) {
        self.saved = Some(self.undo.len());
        self.sealed = true;
    }

    pub fn is_dirty(&self) -> bool {
        self.saved != Some(self.undo.len())
    }

    fn can_merge(&self, kind: Kind) -> bool {
        !self.sealed && self.undo.last().is_some_and(|e| e.kind == kind)
    }

    fn push(&mut self, mut edit: Edit) {
        edit.group = self.open_group.unwrap_or_else(|| {
            self.last_group += 1;
            self.last_group
        });
        // Branching off after undoing past the save point: it can't be reached again.
        if self.saved.is_some_and(|s| self.undo.len() < s) {
            self.saved = None;
        }
        self.undo.push(edit);
        self.redo.clear();
        self.sealed = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_str(h: &mut History, buf: &mut Buffer, mut at: Pos, s: &str) -> Pos {
        for c in s.chars() {
            at = h.insert(buf, at, &c.to_string(), at, true);
        }
        at
    }

    #[test]
    fn undoes_typing_word_by_word() {
        let mut buf = Buffer::new();
        let mut h = History::new();
        type_str(&mut h, &mut buf, Pos::new(0, 0), "hello world");
        assert!(h.is_dirty());
        h.undo(&mut buf);
        assert_eq!(buf.line(0), "hello");
        h.undo(&mut buf);
        assert_eq!(buf.line(0), "");
        assert!(!h.is_dirty());
        h.redo(&mut buf);
        h.redo(&mut buf);
        assert_eq!(buf.line(0), "hello world");
    }

    #[test]
    fn merges_backspaces() {
        let mut buf = Buffer::from_text("abc");
        let mut h = History::new();
        for col in (1..=3).rev() {
            let (s, e) = (Pos::new(0, col - 1), Pos::new(0, col));
            h.delete(&mut buf, s, e, e, true);
        }
        assert_eq!(buf.line(0), "");
        h.undo(&mut buf);
        assert_eq!(buf.line(0), "abc");
    }

    #[test]
    fn tracks_the_save_point() {
        let mut buf = Buffer::new();
        let mut h = History::new();
        type_str(&mut h, &mut buf, Pos::new(0, 0), "a");
        h.mark_saved();
        type_str(&mut h, &mut buf, Pos::new(0, 1), "b");
        assert!(h.is_dirty());
        h.undo(&mut buf);
        assert!(!h.is_dirty());
        h.undo(&mut buf);
        type_str(&mut h, &mut buf, Pos::new(0, 0), "c");
        assert!(h.is_dirty());
    }

    #[test]
    fn undoes_a_group_in_one_step() {
        let mut buf = Buffer::from_text("one two");
        let mut h = History::new();
        h.begin_group();
        h.delete(
            &mut buf,
            Pos::new(0, 4),
            Pos::new(0, 7),
            Pos::new(0, 7),
            false,
        );
        h.insert(&mut buf, Pos::new(0, 4), "2", Pos::new(0, 7), false);
        h.delete(
            &mut buf,
            Pos::new(0, 0),
            Pos::new(0, 3),
            Pos::new(0, 7),
            false,
        );
        h.insert(&mut buf, Pos::new(0, 0), "1", Pos::new(0, 7), false);
        h.end_group();
        assert_eq!(buf.line(0), "1 2");
        assert_eq!(h.undo(&mut buf), Some(Pos::new(0, 7)));
        assert_eq!(buf.line(0), "one two");
        h.redo(&mut buf);
        assert_eq!(buf.line(0), "1 2");
    }
}

//! Lightweight syntax highlighting: keywords, strings, comments and numbers.
//! Colours are the terminal's own 16, so they follow its theme.

use std::path::Path;

use crate::buffer::Buffer;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Text,
    Comment,
    String,
    Number,
    Keyword,
    Type,
    Function,
    Constant,
    Heading,
}

/// What a line starts inside of, carried over from the line before.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    #[default]
    Normal,
    Comment,
    /// Inside a string opened with `Lang::strings[i]`.
    Str(u8),
    /// Inside a Markdown fenced code block.
    Code,
}

pub struct Lang {
    pub name: &'static str,
    line_comments: &'static [&'static str],
    block_comment: Option<(&'static str, &'static str)>,
    /// String delimiters, longest first (`"""` before `"`).
    strings: &'static [&'static str],
    multiline_strings: bool,
    keywords: &'static [&'static str],
    types: &'static [&'static str],
    constants: &'static [&'static str],
    /// Words starting with an uppercase letter are types.
    capitalized_types: bool,
    /// `#` starts a comment only at the start of a word (shells).
    hash_needs_space: bool,
    /// `$name` is a variable (shells).
    dollar_vars: bool,
    /// `-` can be part of a word (`rust-version`, `font-size`).
    dash_in_words: bool,
    /// The first word of a line followed by this is a key (TOML `key =`, YAML `key:`).
    key_suffix: Option<char>,
    /// The first word of every line is highlighted (KDL nodes).
    first_word: bool,
    /// `'` starts a character literal or a lifetime, and `name!` is a macro (Rust).
    rust_quirks: bool,
    markdown: bool,
}

const BASE: Lang = Lang {
    name: "",
    line_comments: &[],
    block_comment: None,
    strings: &[],
    multiline_strings: false,
    keywords: &[],
    types: &[],
    constants: &[],
    capitalized_types: false,
    hash_needs_space: false,
    dollar_vars: false,
    dash_in_words: false,
    key_suffix: None,
    first_word: false,
    rust_quirks: false,
    markdown: false,
};

const C_COMMENTS: &[&str] = &["//"];
const C_BLOCK: Option<(&str, &str)> = Some(("/*", "*/"));

static RUST: Lang = Lang {
    name: "Rust",
    line_comments: C_COMMENTS,
    block_comment: C_BLOCK,
    strings: &["\""],
    multiline_strings: true,
    keywords: &[
        "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
        "extern", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut",
        "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait", "type",
        "unsafe", "use", "where", "while", "yield",
    ],
    types: &[
        "bool", "char", "str", "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32",
        "i64", "i128", "isize", "f32", "f64",
    ],
    constants: &["true", "false", "None", "Some", "Ok", "Err"],
    capitalized_types: true,
    rust_quirks: true,
    ..BASE
};

static PYTHON: Lang = Lang {
    name: "Python",
    line_comments: &["#"],
    strings: &["\"\"\"", "'''", "\"", "'"],
    multiline_strings: true,
    keywords: &[
        "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del",
        "elif", "else", "except", "finally", "for", "from", "global", "if", "import", "in", "is",
        "lambda", "match", "case", "nonlocal", "not", "or", "pass", "raise", "return", "try",
        "while", "with", "yield", "self",
    ],
    types: &[
        "int", "float", "str", "bool", "list", "dict", "set", "tuple", "bytes", "object",
    ],
    constants: &["True", "False", "None"],
    capitalized_types: true,
    ..BASE
};

static JAVASCRIPT: Lang = Lang {
    name: "JavaScript",
    line_comments: C_COMMENTS,
    block_comment: C_BLOCK,
    strings: &["\"", "'", "`"],
    multiline_strings: true,
    keywords: &[
        "async",
        "await",
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "default",
        "delete",
        "do",
        "else",
        "export",
        "extends",
        "finally",
        "for",
        "from",
        "function",
        "if",
        "import",
        "in",
        "instanceof",
        "let",
        "new",
        "of",
        "return",
        "static",
        "super",
        "switch",
        "this",
        "throw",
        "try",
        "typeof",
        "var",
        "void",
        "while",
        "yield",
        "interface",
        "type",
        "enum",
        "implements",
        "private",
        "public",
        "protected",
        "readonly",
        "as",
    ],
    types: &[
        "string", "number", "boolean", "any", "unknown", "never", "object", "void",
    ],
    constants: &["true", "false", "null", "undefined", "NaN"],
    capitalized_types: true,
    ..BASE
};

static C: Lang = Lang {
    name: "C/C++",
    line_comments: C_COMMENTS,
    block_comment: C_BLOCK,
    strings: &["\"", "'"],
    keywords: &[
        "auto",
        "break",
        "case",
        "class",
        "const",
        "continue",
        "default",
        "delete",
        "do",
        "else",
        "enum",
        "extern",
        "for",
        "goto",
        "if",
        "inline",
        "namespace",
        "new",
        "private",
        "protected",
        "public",
        "return",
        "sizeof",
        "static",
        "struct",
        "switch",
        "template",
        "this",
        "typedef",
        "union",
        "using",
        "virtual",
        "volatile",
        "while",
        "#include",
        "#define",
        "#ifdef",
        "#ifndef",
        "#endif",
        "#if",
        "#else",
        "#pragma",
    ],
    types: &[
        "void", "int", "char", "short", "long", "float", "double", "signed", "unsigned", "bool",
        "size_t", "uint8_t", "uint16_t", "uint32_t", "uint64_t", "int8_t", "int16_t", "int32_t",
        "int64_t",
    ],
    constants: &["true", "false", "NULL", "nullptr"],
    ..BASE
};

static GO: Lang = Lang {
    name: "Go",
    line_comments: C_COMMENTS,
    block_comment: C_BLOCK,
    strings: &["\"", "'", "`"],
    keywords: &[
        "break",
        "case",
        "chan",
        "const",
        "continue",
        "default",
        "defer",
        "else",
        "fallthrough",
        "for",
        "func",
        "go",
        "goto",
        "if",
        "import",
        "interface",
        "map",
        "package",
        "range",
        "return",
        "select",
        "struct",
        "switch",
        "type",
        "var",
    ],
    types: &[
        "bool", "byte", "error", "float32", "float64", "int", "int8", "int16", "int32", "int64",
        "rune", "string", "uint", "uint8", "uint16", "uint32", "uint64", "any",
    ],
    constants: &["true", "false", "nil", "iota"],
    capitalized_types: true,
    ..BASE
};

static SHELL: Lang = Lang {
    name: "Shell",
    line_comments: &["#"],
    strings: &["\"", "'"],
    multiline_strings: true,
    keywords: &[
        "if", "then", "else", "elif", "fi", "for", "while", "until", "do", "done", "case", "esac",
        "in", "function", "return", "local", "export", "readonly", "source", "exit", "set",
        "unset", "shift", "break", "continue",
    ],
    constants: &["true", "false"],
    hash_needs_space: true,
    dollar_vars: true,
    dash_in_words: true,
    ..BASE
};

static FISH: Lang = Lang {
    name: "Fish",
    line_comments: &["#"],
    strings: &["\"", "'"],
    multiline_strings: true,
    keywords: &[
        "if", "else", "end", "for", "in", "while", "switch", "case", "function", "return", "and",
        "or", "not", "begin", "set", "abbr", "alias", "source", "exit", "break", "continue",
    ],
    constants: &["true", "false"],
    hash_needs_space: true,
    dollar_vars: true,
    dash_in_words: true,
    ..BASE
};

static LUA: Lang = Lang {
    name: "Lua",
    line_comments: &["--"],
    strings: &["\"", "'"],
    keywords: &[
        "and", "break", "do", "else", "elseif", "end", "for", "function", "goto", "if", "in",
        "local", "not", "or", "repeat", "return", "then", "until", "while",
    ],
    constants: &["true", "false", "nil"],
    ..BASE
};

static SQL: Lang = Lang {
    name: "SQL",
    line_comments: &["--"],
    block_comment: C_BLOCK,
    strings: &["'", "\""],
    keywords: &[
        "select",
        "from",
        "where",
        "insert",
        "into",
        "values",
        "update",
        "set",
        "delete",
        "create",
        "table",
        "alter",
        "drop",
        "index",
        "view",
        "join",
        "left",
        "right",
        "inner",
        "outer",
        "on",
        "and",
        "or",
        "not",
        "in",
        "is",
        "as",
        "order",
        "by",
        "group",
        "having",
        "limit",
        "offset",
        "primary",
        "key",
        "foreign",
        "references",
        "unique",
        "default",
        "returns",
        "return",
        "function",
        "begin",
        "end",
        "if",
        "exists",
        "with",
        "union",
        "distinct",
        "case",
        "when",
        "then",
        "else",
        "policy",
        "grant",
        "using",
        "check",
        "SELECT",
        "FROM",
        "WHERE",
        "INSERT",
        "INTO",
        "VALUES",
        "UPDATE",
        "SET",
        "DELETE",
        "CREATE",
        "TABLE",
        "ALTER",
        "DROP",
        "INDEX",
        "VIEW",
        "JOIN",
        "LEFT",
        "RIGHT",
        "INNER",
        "OUTER",
        "ON",
        "AND",
        "OR",
        "NOT",
        "IN",
        "IS",
        "AS",
        "ORDER",
        "BY",
        "GROUP",
        "HAVING",
        "LIMIT",
        "OFFSET",
        "PRIMARY",
        "KEY",
        "FOREIGN",
        "REFERENCES",
        "UNIQUE",
        "DEFAULT",
        "RETURNS",
        "RETURN",
        "FUNCTION",
        "BEGIN",
        "END",
        "IF",
        "EXISTS",
        "WITH",
        "UNION",
        "DISTINCT",
        "CASE",
        "WHEN",
        "THEN",
        "ELSE",
        "POLICY",
        "GRANT",
        "USING",
        "CHECK",
    ],
    types: &[
        "int",
        "integer",
        "bigint",
        "text",
        "varchar",
        "boolean",
        "uuid",
        "timestamp",
        "timestamptz",
        "date",
        "numeric",
        "jsonb",
        "json",
        "serial",
        "INT",
        "INTEGER",
        "BIGINT",
        "TEXT",
        "VARCHAR",
        "BOOLEAN",
        "UUID",
        "TIMESTAMP",
        "TIMESTAMPTZ",
        "DATE",
        "NUMERIC",
        "JSONB",
        "JSON",
        "SERIAL",
    ],
    constants: &["true", "false", "null", "TRUE", "FALSE", "NULL"],
    ..BASE
};

static CSS: Lang = Lang {
    name: "CSS",
    block_comment: C_BLOCK,
    strings: &["\"", "'"],
    constants: &["important", "inherit", "initial", "none", "auto"],
    dash_in_words: true,
    key_suffix: Some(':'),
    ..BASE
};

static TOML: Lang = Lang {
    name: "TOML",
    line_comments: &["#"],
    strings: &["\"\"\"", "'''", "\"", "'"],
    constants: &["true", "false"],
    dash_in_words: true,
    key_suffix: Some('='),
    ..BASE
};

static INI: Lang = Lang {
    name: "Config",
    line_comments: &["#", ";"],
    strings: &["\"", "'"],
    constants: &["true", "false", "yes", "no", "on", "off"],
    dash_in_words: true,
    key_suffix: Some('='),
    ..BASE
};

static YAML: Lang = Lang {
    name: "YAML",
    line_comments: &["#"],
    strings: &["\"", "'"],
    constants: &["true", "false", "null", "yes", "no", "on", "off"],
    hash_needs_space: true,
    dash_in_words: true,
    key_suffix: Some(':'),
    ..BASE
};

static JSON: Lang = Lang {
    name: "JSON",
    line_comments: C_COMMENTS,
    block_comment: C_BLOCK,
    strings: &["\""],
    constants: &["true", "false", "null"],
    ..BASE
};

static KDL: Lang = Lang {
    name: "KDL",
    line_comments: C_COMMENTS,
    block_comment: C_BLOCK,
    strings: &["\""],
    multiline_strings: true,
    constants: &["true", "false", "null", "#true", "#false", "#null"],
    dash_in_words: true,
    first_word: true,
    ..BASE
};

static MARKDOWN: Lang = Lang {
    name: "Markdown",
    markdown: true,
    ..BASE
};

/// Picks the language from the file name, then from a `#!` line.
pub fn detect(path: &Path, first_line: &str) -> Option<&'static Lang> {
    let name = path.file_name()?.to_str()?;
    let by_name = match name {
        "PKGBUILD" | ".bashrc" | ".bash_profile" | ".zshrc" | ".profile" | ".xprofile" => {
            Some(&SHELL)
        }
        "Cargo.lock" => Some(&TOML),
        _ => None,
    };
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    let by_ext = || -> Option<&'static Lang> {
        Some(match ext.as_deref()? {
            "rs" => &RUST,
            "py" | "pyw" => &PYTHON,
            "js" | "mjs" | "cjs" | "jsx" | "ts" | "tsx" | "mts" => &JAVASCRIPT,
            "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" | "hh" => &C,
            "go" => &GO,
            "sh" | "bash" | "zsh" => &SHELL,
            "fish" => &FISH,
            "lua" => &LUA,
            "sql" => &SQL,
            "css" | "scss" => &CSS,
            "toml" => &TOML,
            "ini" | "conf" | "cfg" | "service" | "timer" | "socket" | "desktop" | "env" => &INI,
            "yaml" | "yml" => &YAML,
            "json" | "jsonc" => &JSON,
            "kdl" => &KDL,
            "md" | "markdown" => &MARKDOWN,
            _ => return None,
        })
    };
    let by_shebang = || -> Option<&'static Lang> {
        let line = first_line.strip_prefix("#!")?;
        Some(if line.contains("fish") {
            &FISH
        } else if line.contains("python") {
            &PYTHON
        } else if line.contains("node") {
            &JAVASCRIPT
        } else if line.contains("sh") {
            &SHELL
        } else {
            return None;
        })
    };
    by_name.or_else(by_ext).or_else(by_shebang)
}

/// Highlights one line, given the state it starts in.
/// Returns a kind per character and the state the next line starts in.
pub fn highlight(lang: &Lang, line: &str, state: State) -> (Vec<Kind>, State) {
    if lang.markdown {
        markdown(line, state)
    } else {
        code(lang, line, state)
    }
}

fn code(lang: &Lang, line: &str, mut state: State) -> (Vec<Kind>, State) {
    let c: Vec<char> = line.chars().collect();
    let n = c.len();
    let mut kinds = vec![Kind::Text; n];
    let is_word = |ch: char| ch.is_alphanumeric() || ch == '_' || (lang.dash_in_words && ch == '-');
    let mut first_word = true;
    let mut i = 0;
    while i < n {
        match state {
            State::Comment => {
                let close = lang.block_comment.map_or("", |(_, close)| close);
                let end = match find(&c, i, close) {
                    Some(at) => {
                        state = State::Normal;
                        at + close.len()
                    }
                    None => n,
                };
                kinds[i..end].fill(Kind::Comment);
                i = end;
                continue;
            }
            State::Str(s) => {
                let delim = lang.strings[s as usize];
                let mut j = i;
                while j < n {
                    if c[j] == '\\' {
                        j += 2;
                    } else if at(&c, j, delim) {
                        j += delim.len();
                        state = State::Normal;
                        break;
                    } else {
                        j += 1;
                    }
                }
                let j = j.min(n);
                kinds[i..j].fill(Kind::String);
                i = j;
                continue;
            }
            State::Normal | State::Code => {}
        }

        let ch = c[i];
        let word_start = i == 0 || c[i - 1].is_whitespace();
        if lang.line_comments.iter().any(|p| at(&c, i, p))
            && !(lang.hash_needs_space && ch == '#' && !word_start)
        {
            kinds[i..].fill(Kind::Comment);
            break;
        }
        if let Some((open, _)) = lang.block_comment
            && at(&c, i, open)
        {
            kinds[i..i + open.len()].fill(Kind::Comment);
            i += open.len();
            state = State::Comment;
            continue;
        }
        if lang.rust_quirks && ch == '\'' {
            // 'a' or '\n' is a character, 'a alone is a lifetime.
            let end = if c.get(i + 1) == Some(&'\\') {
                (i + 2..n.min(i + 12))
                    .find(|&j| c[j] == '\'')
                    .map(|j| j + 1)
            } else if c.get(i + 2) == Some(&'\'') {
                Some(i + 3)
            } else {
                None
            };
            i = match end {
                Some(end) => {
                    kinds[i..end].fill(Kind::String);
                    end
                }
                None => {
                    let end = (i + 1..n).find(|&j| !is_word(c[j])).unwrap_or(n);
                    kinds[i..end].fill(Kind::Type);
                    end
                }
            };
            first_word = false;
            continue;
        }
        if let Some(s) = lang.strings.iter().position(|d| at(&c, i, d)) {
            let len = lang.strings[s].len();
            kinds[i..i + len].fill(Kind::String);
            i += len;
            state = State::Str(s as u8);
            first_word = false;
            continue;
        }
        if lang.dollar_vars && ch == '$' {
            let end = (i + 1..n)
                .find(|&j| !(c[j].is_alphanumeric() || c[j] == '_'))
                .unwrap_or(n);
            kinds[i..end].fill(Kind::Constant);
            i = end.max(i + 1);
            continue;
        }
        if ch.is_ascii_digit() && (i == 0 || !is_word(c[i - 1])) {
            let mut j = i + 1;
            while j < n
                && (c[j].is_alphanumeric()
                    || c[j] == '_'
                    || (c[j] == '.' && c.get(j + 1).is_some_and(char::is_ascii_digit)))
            {
                j += 1;
            }
            kinds[i..j].fill(Kind::Number);
            i = j;
            first_word = false;
            continue;
        }
        // `#` is allowed so C preprocessor lines and KDL's #true are one word.
        if ch.is_alphabetic()
            || ch == '_'
            || (ch == '#' && c.get(i + 1).is_some_and(|x| x.is_alphabetic()))
        {
            let mut j = i + 1;
            while j < n && is_word(c[j]) {
                j += 1;
            }
            let word: String = c[i..j].iter().collect();
            let next = c[j..].iter().copied().find(|x| !x.is_whitespace());
            let kind = if lang.keywords.contains(&word.as_str()) {
                Kind::Keyword
            } else if lang.constants.contains(&word.as_str()) {
                Kind::Constant
            } else if lang.types.contains(&word.as_str()) {
                Kind::Type
            } else if first_word
                && (lang.first_word || lang.key_suffix.is_some_and(|s| next == Some(s)))
            {
                Kind::Function
            } else if lang.capitalized_types && ch.is_uppercase() {
                Kind::Type
            } else if next == Some('(') || (lang.rust_quirks && c.get(j) == Some(&'!')) {
                Kind::Function
            } else {
                Kind::Text
            };
            kinds[i..j].fill(kind);
            i = j;
            first_word = false;
            continue;
        }
        if !ch.is_whitespace() && !matches!(ch, '-' | '[' | '.') {
            first_word = false;
        }
        i += 1;
    }
    if matches!(state, State::Str(_)) && !lang.multiline_strings {
        state = State::Normal;
    }
    (kinds, state)
}

fn markdown(line: &str, state: State) -> (Vec<Kind>, State) {
    let n = line.chars().count();
    let trimmed = line.trim_start();
    if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
        let next = if state == State::Code {
            State::Normal
        } else {
            State::Code
        };
        return (vec![Kind::String; n], next);
    }
    if state == State::Code {
        return (vec![Kind::String; n], state);
    }
    if trimmed.starts_with('#') {
        return (vec![Kind::Heading; n], state);
    }
    if trimmed.starts_with('>') {
        return (vec![Kind::Comment; n], state);
    }
    let c: Vec<char> = line.chars().collect();
    let mut kinds = vec![Kind::Text; n];
    let indent = n - trimmed.chars().count();
    let marker = ["- ", "* ", "+ ", "- [ ] ", "- [x] "]
        .iter()
        .filter(|m| trimmed.starts_with(*m))
        .map(|m| m.len())
        .max()
        .or_else(|| {
            let digits = trimmed.chars().take_while(char::is_ascii_digit).count();
            (digits > 0 && trimmed[digits..].starts_with(". ")).then_some(digits + 2)
        });
    if let Some(len) = marker {
        kinds[indent..indent + len].fill(Kind::Keyword);
    }
    // `inline code`
    let mut i = 0;
    while i < n {
        if c[i] == '`'
            && let Some(end) = (i + 1..n).find(|&j| c[j] == '`')
        {
            kinds[i..=end].fill(Kind::String);
            i = end + 1;
        } else {
            i += 1;
        }
    }
    (kinds, state)
}

/// Whether `pat` (ASCII) starts at `c[i]`.
fn at(c: &[char], i: usize, pat: &str) -> bool {
    !pat.is_empty() && i + pat.len() <= c.len() && pat.chars().zip(&c[i..]).all(|(p, x)| p == *x)
}

fn find(c: &[char], from: usize, pat: &str) -> Option<usize> {
    (from..c.len()).find(|&i| at(c, i, pat))
}

/// Highlights lines on demand and caches the state at the start of each line,
/// so only the lines that changed (and those after them) are re-scanned.
pub struct Highlighter {
    lang: &'static Lang,
    /// `states[i]` is the state line `i` starts in.
    states: Vec<State>,
}

impl Highlighter {
    pub fn new(lang: &'static Lang) -> Self {
        Self {
            lang,
            states: vec![State::Normal],
        }
    }

    pub fn name(&self) -> &'static str {
        self.lang.name
    }

    /// Forgets everything from `line` on, after an edit there.
    pub fn invalidate(&mut self, line: usize) {
        self.states.truncate(line + 1);
    }

    pub fn line(&mut self, buf: &Buffer, i: usize) -> Vec<Kind> {
        while self.states.len() <= i {
            let j = self.states.len() - 1;
            let (_, next) = highlight(self.lang, buf.line(j), self.states[j]);
            self.states.push(next);
        }
        highlight(self.lang, buf.line(i), self.states[i]).0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds_of(lang: &Lang, line: &str) -> Vec<(String, Kind)> {
        let (kinds, _) = highlight(lang, line, State::Normal);
        let mut out: Vec<(String, Kind)> = Vec::new();
        for (ch, kind) in line.chars().zip(kinds) {
            match out.last_mut() {
                Some((s, k)) if *k == kind => s.push(ch),
                _ => out.push((ch.to_string(), kind)),
            }
        }
        out.into_iter()
            .map(|(s, k)| (s.trim().to_owned(), k))
            .filter(|(s, _)| !s.is_empty())
            .collect()
    }

    #[test]
    fn highlights_rust() {
        let got = kinds_of(&RUST, r#"fn main() { let s: &'a str = "hi"; } // done"#);
        assert!(got.contains(&("fn".into(), Kind::Keyword)));
        assert!(got.contains(&("main".into(), Kind::Function)));
        assert!(got.contains(&("'a".into(), Kind::Type)));
        assert!(got.contains(&("\"hi\"".into(), Kind::String)));
        assert!(got.contains(&("// done".into(), Kind::Comment)));
    }

    #[test]
    fn carries_block_comments_across_lines() {
        let (_, state) = highlight(&RUST, "let x = 1; /* start", State::Normal);
        assert_eq!(state, State::Comment);
        let (kinds, state) = highlight(&RUST, "end */ y", state);
        assert_eq!(kinds[0], Kind::Comment);
        assert_eq!(kinds[7], Kind::Text);
        assert_eq!(state, State::Normal);
    }

    #[test]
    fn shell_hash_inside_word_is_not_a_comment() {
        let got = kinds_of(&SHELL, "echo a#b $HOME # note");
        assert!(got.contains(&("echo a#b".into(), Kind::Text)));
        assert!(got.contains(&("$HOME".into(), Kind::Constant)));
        assert!(got.contains(&("# note".into(), Kind::Comment)));
    }

    #[test]
    fn detects_languages() {
        assert_eq!(
            detect(Path::new("main.rs"), "").map(|l| l.name),
            Some("Rust")
        );
        assert_eq!(
            detect(Path::new("config.fish"), "").map(|l| l.name),
            Some("Fish")
        );
        assert_eq!(
            detect(Path::new("script"), "#!/usr/bin/env python3").map(|l| l.name),
            Some("Python")
        );
        assert_eq!(
            detect(Path::new("notes.txt"), "hello").map(|l| l.name),
            None
        );
    }
}

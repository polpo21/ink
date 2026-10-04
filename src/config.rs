//! User settings from `~/.config/ink/config.toml` (or `$XDG_CONFIG_HOME/ink/config.toml`).
//! Every setting is optional; a missing file means the defaults.

use std::{collections::HashMap, env, fs, io, path::PathBuf, str::FromStr};

use ratatui::style::Color;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Columns a tab takes on screen.
    pub tab_width: usize,
    /// Insert spaces instead of a tab character when pressing Tab.
    pub tabs_to_spaces: bool,
    /// Keep the indentation of the current line when pressing Enter.
    pub auto_indent: bool,
    pub line_numbers: bool,
    /// Capture the mouse (click, drag, wheel). Turn off to use the terminal's own selection.
    pub mouse: bool,
    /// Lines moved by one step of the mouse wheel.
    pub scroll_lines: usize,
    pub colors: Colors,
    /// Per-language overrides, keyed by language name in lowercase (`python`, `go`, …).
    pub languages: HashMap<String, Indent>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tab_width: 4,
            tabs_to_spaces: false,
            auto_indent: true,
            line_numbers: true,
            mouse: true,
            scroll_lines: 3,
            colors: Colors::default(),
            languages: HashMap::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Indent {
    pub tab_width: Option<usize>,
    pub tabs_to_spaces: Option<bool>,
}

/// Colours: a name (`red`, `lightblue`, `darkgray`…), a palette index (`"8"`) or `#rrggbb`.
/// Names and indexes use the terminal's own palette, so they follow its theme.
#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Colors {
    #[serde(deserialize_with = "color")]
    pub comment: Color,
    #[serde(deserialize_with = "color")]
    pub string: Color,
    #[serde(deserialize_with = "color")]
    pub number: Color,
    #[serde(deserialize_with = "color")]
    pub constant: Color,
    #[serde(deserialize_with = "color")]
    pub keyword: Color,
    #[serde(deserialize_with = "color", rename = "type")]
    pub type_: Color,
    #[serde(deserialize_with = "color")]
    pub function: Color,
    #[serde(deserialize_with = "color")]
    pub heading: Color,
    #[serde(deserialize_with = "color")]
    pub line_number: Color,
    #[serde(deserialize_with = "color")]
    pub current_line_number: Color,
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            comment: Color::DarkGray,
            string: Color::Green,
            number: Color::Cyan,
            constant: Color::Cyan,
            keyword: Color::Magenta,
            type_: Color::Yellow,
            function: Color::Blue,
            heading: Color::Blue,
            line_number: Color::DarkGray,
            current_line_number: Color::Yellow,
        }
    }
}

fn color<'de, D: Deserializer<'de>>(de: D) -> Result<Color, D::Error> {
    let s = String::deserialize(de)?;
    Color::from_str(&s).map_err(|_| serde::de::Error::custom(format!("unknown colour \"{s}\"")))
}

impl Config {
    pub fn path() -> Option<PathBuf> {
        let base = env::var_os("XDG_CONFIG_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
        Some(base.join("ink").join("config.toml"))
    }

    /// Loads the config. On a broken file, returns the defaults and the error
    /// to show, so a typo never stops the editor from opening.
    pub fn load() -> (Self, Option<String>) {
        let Some(path) = Self::path() else {
            return (Self::default(), None);
        };
        match fs::read_to_string(&path) {
            Ok(text) => match Self::parse(&text) {
                Ok(config) => (config, None),
                Err(e) => (
                    Self::default(),
                    Some(format!("Config error, using defaults: {e}")),
                ),
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => (Self::default(), None),
            Err(e) => (
                Self::default(),
                Some(format!("Cannot read {}: {e}", path.display())),
            ),
        }
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let config: Self = toml::from_str(text).map_err(|e| e.message().to_owned())?;
        let widths = std::iter::once(config.tab_width)
            .chain(config.languages.values().filter_map(|l| l.tab_width));
        if widths.into_iter().any(|w| !(1..=16).contains(&w)) {
            return Err("tab_width must be between 1 and 16".into());
        }
        Ok(config)
    }

    /// Tab width and tabs-to-spaces for a language, with its overrides applied.
    pub fn indent_for(&self, lang: Option<&str>) -> (usize, bool) {
        let o = lang
            .and_then(|l| self.languages.get(&l.to_lowercase()))
            .copied()
            .unwrap_or_default();
        (
            o.tab_width.unwrap_or(self.tab_width),
            o.tabs_to_spaces.unwrap_or(self.tabs_to_spaces),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_file_gives_defaults() {
        let config = Config::parse("").unwrap();
        assert_eq!(config.tab_width, 4);
        assert!(config.mouse);
    }

    #[test]
    fn reads_settings_and_language_overrides() {
        let config = Config::parse(
            r##"
            tab_width = 8
            mouse = false
            [colors]
            keyword = "red"
            string = "#a0c070"
            [languages.python]
            tabs_to_spaces = true
            tab_width = 4
            "##,
        )
        .unwrap();
        assert!(!config.mouse);
        assert_eq!(config.colors.keyword, Color::Red);
        assert_eq!(
            Colors::default().comment,
            Color::from_str("darkgray").unwrap()
        );
        assert_eq!(config.colors.string, Color::Rgb(0xa0, 0xc0, 0x70));
        assert_eq!(config.indent_for(Some("Python")), (4, true));
        assert_eq!(config.indent_for(Some("Go")), (8, false));
        assert_eq!(config.indent_for(None), (8, false));
    }

    #[test]
    fn rejects_mistakes() {
        assert!(Config::parse("tab_widht = 2").is_err());
        assert!(Config::parse("tab_width = 0").is_err());
        assert!(Config::parse("[colors]\nkeyword = \"purplish\"").is_err());
    }
}

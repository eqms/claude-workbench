use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use std::path::Path;
use syntect::{
    easy::HighlightLines,
    highlighting::{FontStyle, ThemeSet},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

pub struct SyntaxManager {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    theme_name: String,
}

impl Default for SyntaxManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SyntaxManager {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            theme_name: "base16-ocean.dark".to_string(),
        }
    }

    /// Detect syntax name via central registry
    pub fn detect_syntax_name(&self, path: &Path) -> Option<String> {
        Some(crate::syntax_registry::display_name_for_path(
            path,
            &self.syntax_set,
        ))
    }

    /// Highlight content and return ratatui Lines
    pub fn highlight(&self, content: &str, path: &Path) -> Vec<Line<'static>> {
        let syntax = crate::syntax_registry::find_syntax_for_path(path, &self.syntax_set);

        let theme = self
            .theme_set
            .themes
            .get(&self.theme_name)
            .unwrap_or_else(|| {
                self.theme_set
                    .themes
                    .values()
                    .next()
                    .expect("No themes available")
            });

        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut lines: Vec<Line<'static>> = Vec::new();

        for line in LinesWithEndings::from(content) {
            match highlighter.highlight_line(line, &self.syntax_set) {
                Ok(ranges) => {
                    let spans: Vec<Span<'static>> = ranges
                        .iter()
                        .map(|(style, text)| {
                            let fg = syntect_color_to_ratatui(style.foreground);
                            let mut ratatui_style = Style::default().fg(fg);

                            if style.font_style.contains(FontStyle::BOLD) {
                                ratatui_style =
                                    ratatui_style.add_modifier(ratatui::style::Modifier::BOLD);
                            }
                            if style.font_style.contains(FontStyle::ITALIC) {
                                ratatui_style =
                                    ratatui_style.add_modifier(ratatui::style::Modifier::ITALIC);
                            }
                            if style.font_style.contains(FontStyle::UNDERLINE) {
                                ratatui_style = ratatui_style
                                    .add_modifier(ratatui::style::Modifier::UNDERLINED);
                            }

                            Span::styled(text.to_string(), ratatui_style)
                        })
                        .collect();

                    lines.push(Line::from(spans));
                }
                Err(_) => {
                    // Fallback: plain text line
                    lines.push(Line::from(line.to_string()));
                }
            }
        }

        lines
    }

    /// Highlight plain text without syntax (fallback)
    pub fn plain_text(&self, content: &str) -> Vec<Line<'static>> {
        content.lines().map(|l| Line::from(l.to_string())).collect()
    }
}

/// Convert syntect color to ratatui Color
fn syntect_color_to_ratatui(color: syntect::highlighting::Color) -> Color {
    Color::Rgb(color.r, color.g, color.b)
}

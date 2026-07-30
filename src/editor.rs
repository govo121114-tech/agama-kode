use ratatui::layout::Rect;
use ratatui::style::{Style, Modifier};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};
use ratatui::widgets::Borders;
use syntect::highlighting::ThemeSet;

use syntect::parsing::SyntaxSet;
use syntect::easy::HighlightLines;
use crate::buffer::{TextBuffer, SearchMatch};
use crate::theme;
use crate::search::SearchState;

pub struct EditorRenderer {
    syntax_set: SyntaxSet,
    theme: syntect::highlighting::Theme,
    line_offset: usize,
    col_offset: usize,
}

impl EditorRenderer {
    pub fn new() -> Self {
        let ss = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        let theme = ts.themes["base16-ocean.dark"].clone();
        Self {
            syntax_set: ss,
            theme,
            line_offset: 0,
            col_offset: 0,
        }
    }

    pub fn render<'a>(
        &'a mut self,
        buf: &'a TextBuffer,
        area: Rect,
        search: &SearchState,
    ) -> Paragraph<'a> {
        let line_count = buf.line_count();
        let num_width = line_count.to_string().len().max(3);
        let _available_width = area.width.saturating_sub(num_width as u16 + 3);

        if buf.cursor().line < self.line_offset {
            self.line_offset = buf.cursor().line;
        }
        if buf.cursor().line >= self.line_offset + (area.height as usize).saturating_sub(2) {
            self.line_offset = buf
                .cursor()
                .line
                .saturating_sub(area.height as usize)
                .saturating_add(2);
        }

        let mut lines: Vec<Line> = Vec::new();
        let max_lines = area.height as usize;

        for i in self.line_offset..(self.line_offset + max_lines).min(line_count) {
            let line_num = Span::styled(
                format!("{:>width$} ", i + 1, width = num_width),
                theme::line_num_style(i == buf.cursor().line),
            );

            let content = buf.line(i).unwrap_or("");
            let styled_content = self.highlight_line(buf, i, content, search);

            let spans = std::iter::once(line_num)
                .chain(styled_content)
                .collect::<Vec<_>>();
            lines.push(Line::from(spans));
        }

        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::NONE)
                    .style(Style::default().bg(theme::BG)),
            )
            .wrap(Wrap { trim: false })
    }

    fn highlight_line<'a>(
        &'a self,
        buf: &TextBuffer,
        line_idx: usize,
        content: &'a str,
        search: &SearchState,
    ) -> Vec<Span<'a>> {
        let syntax_name = self.guess_syntax(buf.filename());
        let mut spans: Vec<Span> = Vec::new();

        if content.is_empty() {
            return spans;
        }

        let syntax = self
            .syntax_set
            .find_syntax_by_token(syntax_name)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let mut highlighter = HighlightLines::new(syntax, &self.theme);

        let highlighted = highlighter.highlight_line(content, &self.syntax_set);

        if let Ok(ranges) = highlighted {
            for (style, text) in ranges {
                let fg = style.foreground;
                let color = ratatui::style::Color::Rgb(fg.r, fg.g, fg.b);
                let mut s = Style::default().fg(color);
                if style.font_style.contains(syntect::highlighting::FontStyle::BOLD) {
                    s = s.add_modifier(Modifier::BOLD);
                }
                if style.font_style.contains(syntect::highlighting::FontStyle::ITALIC) {
                    s = s.add_modifier(Modifier::ITALIC);
                }
                if style.font_style.contains(syntect::highlighting::FontStyle::UNDERLINE) {
                    s = s.add_modifier(Modifier::UNDERLINED);
                }
                spans.push(Span::styled(text.to_string(), s));
            }
        } else {
            spans.push(Span::raw(content));
        }

        if !search.query.is_empty() && !search.matches.is_empty() {
            spans = self.apply_search_highlights(spans, content, search, line_idx);
        }

        spans
    }

    fn apply_search_highlights<'a>(
        &self,
        spans: Vec<Span<'a>>,
        content: &'a str,
        search: &SearchState,
        line_idx: usize,
    ) -> Vec<Span<'a>> {
        let line_matches: Vec<&SearchMatch> = search
            .matches
            .iter()
            .filter(|m| m.line == line_idx)
            .collect();

        if line_matches.is_empty() {
            return spans;
        }

        let combined = spans.iter().map(|s| s.content.clone()).collect::<String>();
        if combined.len() != content.len() {
            return spans;
        }

        let mut result = Vec::new();
        let mut pos = 0usize;
        for m in &line_matches {
            if m.start_col > pos {
                result.push(Span::raw(&content[pos..m.start_col]));
            }
            let is_current = search.matches.get(search.current_match).map_or(false, |mm| {
                mm.line == m.line && mm.start_col == m.start_col && mm.end_col == m.end_col
            });
            let style = if is_current {
                Style::default().bg(theme::ACCENT_ORANGE).fg(theme::FG_BRIGHT)
            } else {
                theme::search_match_style()
            };
            result.push(Span::styled(&content[m.start_col..m.end_col], style));
            pos = m.end_col;
        }
        if pos < content.len() {
            result.push(Span::raw(&content[pos..]));
        }
        result
    }

    fn guess_syntax(&self, filename: &str) -> &str {
        if filename.ends_with(".rs") { "Rust" }
        else if filename.ends_with(".py") { "Python" }
        else if filename.ends_with(".js") || filename.ends_with(".jsx") { "JavaScript" }
        else if filename.ends_with(".ts") || filename.ends_with(".tsx") { "TypeScript" }
        else if filename.ends_with(".go") { "Go" }
        else if filename.ends_with(".java") { "Java" }
        else if filename.ends_with(".c") || filename.ends_with(".h") { "C" }
        else if filename.ends_with(".cpp") || filename.ends_with(".hpp") || filename.ends_with(".cc") { "C++" }
        else if filename.ends_with(".rs") { "Rust" }
        else if filename.ends_with(".rb") { "Ruby" }
        else if filename.ends_with(".php") { "PHP" }
        else if filename.ends_with(".swift") { "Swift" }
        else if filename.ends_with(".kt") || filename.ends_with(".kts") { "Kotlin" }
        else if filename.ends_with(".scala") { "Scala" }
        else if filename.ends_with(".ex") || filename.ends_with(".exs") { "Elixir" }
        else if filename.ends_with(".hs") { "Haskell" }
        else if filename.ends_with(".ml") { "OCaml" }
        else if filename.ends_with(".lua") { "Lua" }
        else if filename.ends_with(".r") { "R" }
        else if filename.ends_with(".sh") || filename.ends_with(".bash") { "Shell script" }
        else if filename.ends_with(".sql") { "SQL" }
        else if filename.ends_with(".html") || filename.ends_with(".htm") { "HTML" }
        else if filename.ends_with(".css") || filename.ends_with(".scss") { "CSS" }
        else if filename.ends_with(".json") { "JSON" }
        else if filename.ends_with(".xml") || filename.ends_with(".svg") { "XML" }
        else if filename.ends_with(".yaml") || filename.ends_with(".yml") { "YAML" }
        else if filename.ends_with(".toml") { "TOML" }
        else if filename.ends_with(".md") || filename.ends_with(".markdown") { "Markdown" }
        else if filename.ends_with(".dockerfile") || filename == "Dockerfile" { "Dockerfile" }
        else if filename == "Makefile" || filename.ends_with(".mk") { "Makefile" }
        else if filename == "CMakeLists.txt" || filename.ends_with(".cmake") { "CMake" }
        else { "Plain Text" }
    }
}

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use crate::buffer::TextBuffer;
use crate::theme;

pub struct StatusBar;

impl StatusBar {
    pub fn render<'a>(buf: &'a TextBuffer, mode: &'a str, area: Rect) -> Paragraph<'a> {
        let filename = if buf.is_dirty() {
            format!(" {} *", buf.filename())
        } else {
            format!(" {} ", buf.filename())
        };
        let cursor = buf.cursor();
        let pos = format!(" Ln {}, Col {}", cursor.line + 1, cursor.col + 1);
        let mode_text = format!(" {} ", mode);

        let line_count = buf.line_count();
        let pct = if line_count == 0 {
            100
        } else {
            ((cursor.line + 1) * 100 / line_count).min(100)
        };
        let pct_text = format!(" {}% ", pct);

        let total_width = area.width as usize;
        let mode_len = mode_text.len();
        let fn_len = filename.len();
        let pos_len = pos.len();
        let pct_len = pct_text.len();
        let remaining = total_width.saturating_sub(mode_len + fn_len + pos_len + pct_len + 4);

        let spaces = " ".repeat(remaining);

        let line = Line::from(vec![
            Span::styled(mode_text, Style::default().fg(theme::ACCENT).bg(theme::STATUS_BG)),
            Span::styled(filename, Style::default().fg(theme::FG_BRIGHT).bg(theme::STATUS_BG)),
            Span::styled(spaces, Style::default().bg(theme::STATUS_BG)),
            Span::styled(pos, Style::default().fg(theme::FG_DIM).bg(theme::STATUS_BG)),
            Span::styled(format!(" "), Style::default().bg(theme::STATUS_BG)),
            Span::styled(pct_text, Style::default().fg(theme::FG_DIM).bg(theme::STATUS_BG)),
        ]);

        Paragraph::new(line).style(Style::default().bg(theme::STATUS_BG))
    }
}

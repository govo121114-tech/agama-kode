use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::text::{Line, Span};

#[derive(Clone)]
pub struct SearchState {
    pub query: String,
    pub active: bool,
    pub matches: Vec<crate::buffer::SearchMatch>,
    pub current_match: usize,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            active: false,
            matches: Vec::new(),
            current_match: 0,
        }
    }

    pub fn toggle(&mut self) {
        self.active = !self.active;
        if !self.active {
            self.query.clear();
            self.matches.clear();
            self.current_match = 0;
        }
    }

    pub fn push_char(&mut self, ch: char) {
        self.query.push(ch);
    }

    pub fn pop_char(&mut self) {
        self.query.pop();
    }

    pub fn run_search(&mut self, buffer: &crate::buffer::TextBuffer) {
        self.matches = buffer.find_all(&self.query);
        self.current_match = if self.matches.is_empty() {
            0
        } else {
            self.current_match.min(self.matches.len() - 1)
        };
    }

    pub fn next_match(&mut self) -> Option<&crate::buffer::SearchMatch> {
        if self.matches.is_empty() {
            return None;
        }
        self.current_match = (self.current_match + 1) % self.matches.len();
        Some(&self.matches[self.current_match])
    }

    pub fn prev_match(&mut self) -> Option<&crate::buffer::SearchMatch> {
        if self.matches.is_empty() {
            return None;
        }
        self.current_match = if self.current_match == 0 {
            self.matches.len() - 1
        } else {
            self.current_match - 1
        };
        Some(&self.matches[self.current_match])
    }

    pub fn current_match_info(&self) -> String {
        if self.matches.is_empty() {
            String::from("No results")
        } else {
            format!("{}/{}", self.current_match + 1, self.matches.len())
        }
    }

    pub fn render_search_bar(&self, _area: Rect) -> Paragraph {
        let prefix = Span::raw("/");
        let query = Span::styled(
            &self.query,
            Style::default(),
        );
        let info = Span::styled(
            format!(" {} ", self.current_match_info()),
            Style::default(),
        );
        let text = Line::from(vec![prefix, query, info]);
        Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Search")
                .style(Style::default()),
        )
    }
}

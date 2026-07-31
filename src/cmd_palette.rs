use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};
use ratatui::Frame;
use crate::theme;

pub struct CmdEntry {
    pub name: &'static str,
    pub desc: &'static str,
}

const COMMANDS: &[CmdEntry] = &[
    CmdEntry { name: "terminal", desc: "Toggle integrated terminal" },
    CmdEntry { name: "wsl", desc: "Open WSL (Linux) terminal" },
    CmdEntry { name: "save", desc: "Save current file" },
    CmdEntry { name: "save as", desc: "Save current file with a new name" },
    CmdEntry { name: "reload", desc: "Reload file from disk" },
    CmdEntry { name: "open", desc: "Focus file tree" },
    CmdEntry { name: "new file", desc: "Create a new file tab" },
    CmdEntry { name: "close tab", desc: "Close current tab" },
    CmdEntry { name: "close all", desc: "Close all tabs" },
    CmdEntry { name: "search", desc: "Search in current file" },
    CmdEntry { name: "replace", desc: "Find and replace in file" },
    CmdEntry { name: "select all", desc: "Select entire file content" },
    CmdEntry { name: "go to line", desc: "Jump to a specific line number" },
    CmdEntry { name: "delete line", desc: "Delete the current line" },
    CmdEntry { name: "duplicate line", desc: "Duplicate the current line" },
    CmdEntry { name: "project", desc: "Create new project" },
    CmdEntry { name: "help", desc: "Show keybindings help" },
    CmdEntry { name: "quit", desc: "Exit the editor" },
];

pub struct CmdPalette {
    pub active: bool,
    pub query: String,
    pub selected: usize,
}

impl CmdPalette {
    pub fn new() -> Self {
        Self {
            active: false,
            query: String::new(),
            selected: 0,
        }
    }

    pub fn toggle(&mut self) {
        self.active = !self.active;
        if self.active {
            self.query.clear();
            self.selected = 0;
        }
    }

    pub fn push_char(&mut self, ch: char) {
        self.query.push(ch);
        self.selected = 0;
    }

    pub fn pop_char(&mut self) {
        self.query.pop();
        self.selected = 0;
    }

    pub fn select_next(&mut self) {
        let count = self.filtered().len();
        if count > 0 {
            self.selected = (self.selected + 1) % count;
        }
    }

    pub fn select_prev(&mut self) {
        let count = self.filtered().len();
        if count > 0 {
            self.selected = if self.selected == 0 { count - 1 } else { self.selected - 1 };
        }
    }

    pub fn filtered(&self) -> Vec<&CmdEntry> {
        if self.query.is_empty() {
            return COMMANDS.iter().collect();
        }
        let q = self.query.to_lowercase();
        COMMANDS
            .iter()
            .filter(|c| c.name.contains(&q) || c.desc.to_lowercase().contains(&q))
            .collect()
    }

    pub fn selected_cmd(&self) -> Option<&'static str> {
        let filtered = self.filtered();
        if filtered.is_empty() {
            return None;
        }
        let idx = self.selected.min(filtered.len() - 1);
        Some(filtered[idx].name)
    }

    pub fn render(&self, area: Rect, f: &mut Frame) {
        let w = 50.min(area.width.saturating_sub(8));
        let filtered = self.filtered();
        let h = (filtered.len() as u16 + 3).min(area.height.saturating_sub(4)).min(14);
        let popup = Rect {
            x: (area.width - w) / 2,
            y: 2,
            width: w,
            height: h,
        };
        if popup.width < 10 || popup.height < 3 {
            return;
        }

        f.render_widget(Clear, popup);

        let items: Vec<ListItem> = filtered
            .iter()
            .enumerate()
            .map(|(i, cmd)| {
                let prefix = if i == self.selected { " > " } else { "   " };
                let style = if i == self.selected {
                    Style::default().fg(theme::FG_BRIGHT).bg(theme::BG_LIGHT)
                } else {
                    Style::default().fg(theme::FG)
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{}{}", prefix, cmd.name), style),
                    Span::styled(
                        format!("  — {}", cmd.desc),
                        Style::default().fg(theme::FG_DIM),
                    ),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" Commands: {} ", self.query))
                    .style(Style::default().bg(theme::BG)),
            )
            .highlight_style(Style::default().bg(theme::BG_LIGHT));

        let mut list_state = ListState::default();
        if !filtered.is_empty() {
            list_state.select(Some(self.selected.min(filtered.len() - 1)));
        }
        f.render_stateful_widget(list, popup, &mut list_state);
    }
}

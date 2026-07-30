use std::path::Path;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::text::{Line, Span};
use ratatui::Frame;

use crate::buffer::TextBuffer;
use crate::editor::EditorRenderer;
use crate::filetree::FileTree;
use crate::search::SearchState;
use crate::status::StatusBar;
use crate::theme;

pub enum Mode {
    Normal,
    Command,
    Search,
}

pub struct App {
    pub buffers: Vec<TextBuffer>,
    pub active_buffer: usize,
    pub filetree: FileTree,
    pub search: SearchState,
    pub editor: EditorRenderer,
    pub mode: Mode,
    pub command_buffer: String,
    pub quit: bool,
    pub filetree_focused: bool,
    pub tree_width: u16,
}

impl App {
    pub fn new() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
        let mut app = Self {
            buffers: vec![TextBuffer::new()],
            active_buffer: 0,
            filetree: FileTree::new(&cwd),
            search: SearchState::new(),
            editor: EditorRenderer::new(),
            mode: Mode::Normal,
            command_buffer: String::new(),
            quit: false,
            filetree_focused: false,
            tree_width: 25,
        };
        if let Some(path) = std::env::args().nth(1) {
            app.open_file(&path);
        }
        app
    }

    pub fn active_buf(&mut self) -> &mut TextBuffer {
        let idx = self.active_buffer;
        &mut self.buffers[idx]
    }

    pub fn active_buf_ref(&self) -> &TextBuffer {
        &self.buffers[self.active_buffer]
    }

    pub fn open_file(&mut self, path_str: &str) {
        let path = Path::new(path_str);
        if let Ok(buf) = TextBuffer::from_file(path) {
            if let Some(pos) = self.buffers.iter().position(|b| b.path() == Some(&path.to_path_buf())) {
                self.active_buffer = pos;
                return;
            }
            self.buffers.push(buf);
            self.active_buffer = self.buffers.len() - 1;
        }
    }

    pub fn save_current(&mut self) -> Result<(), String> {
        if self.active_buf().path().is_some() {
            self.active_buf().save()
        } else {
            Err("No path".to_string())
        }
    }

    pub fn save_current_as(&mut self, path_str: &str) -> Result<(), String> {
        let path = Path::new(path_str);
        self.active_buf().save_as(path)
    }

    pub fn close_current_tab(&mut self) {
        if self.buffers.len() > 1 {
            self.buffers.remove(self.active_buffer);
            if self.active_buffer >= self.buffers.len() {
                self.active_buffer = self.buffers.len() - 1;
            }
        }
    }

    pub fn next_tab(&mut self) {
        if self.active_buffer + 1 < self.buffers.len() {
            self.active_buffer += 1;
        }
    }

    pub fn prev_tab(&mut self) {
        if self.active_buffer > 0 {
            self.active_buffer -= 1;
        }
    }

    pub fn handle_event(&mut self, evt: Event) -> Result<(), String> {
        match self.mode {
            Mode::Search => self.handle_search_event(&evt),
            Mode::Command => self.handle_command_event(&evt),
            Mode::Normal => self.handle_normal_event(&evt),
        }
        Ok(())
    }

    fn handle_normal_event(&mut self, evt: &Event) {
        match evt {
            Event::Key(ke) => self.handle_key(ke),
            Event::Mouse(_) => {}
            _ => {}
        }
    }

    fn handle_search_event(&mut self, evt: &Event) {
        match evt {
            Event::Key(ke) => match ke.code {
                KeyCode::Esc => {
                    self.search.toggle();
                    self.mode = Mode::Normal;
                }
                KeyCode::Enter => {
                    let m = self.search.next_match().cloned();
                    if let Some(ref m) = m {
                        self.active_buf().go_to_match(m);
                    }
                }
                KeyCode::Backspace => {
                    self.search.pop_char();
                    let query = self.search.query.clone();
                    self.search.matches = self.active_buf_ref().find_all(&query);
                }
                KeyCode::Char(ch) => {
                    self.search.push_char(ch);
                    let query = self.search.query.clone();
                    self.search.matches = self.active_buf_ref().find_all(&query);
                }
                KeyCode::Tab => {
                    let m = self.search.next_match().cloned();
                    if let Some(ref m) = m {
                        self.active_buf().go_to_match(m);
                    }
                }
                KeyCode::BackTab => {
                    let m = self.search.prev_match().cloned();
                    if let Some(ref m) = m {
                        self.active_buf().go_to_match(m);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn handle_command_event(&mut self, evt: &Event) {
        match evt {
            Event::Key(ke) => match ke.code {
                KeyCode::Esc => {
                    self.mode = Mode::Normal;
                    self.command_buffer.clear();
                }
                KeyCode::Enter => {
                    self.execute_command();
                }
                KeyCode::Backspace => {
                    self.command_buffer.pop();
                }
                KeyCode::Char(ch) => {
                    self.command_buffer.push(ch);
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn execute_command(&mut self) {
        let cmd = self.command_buffer.trim().to_string();
        self.command_buffer.clear();
        self.mode = Mode::Normal;

        if cmd.starts_with("w ") || cmd.starts_with("write ") {
            let path = cmd.split_at(2).1.trim();
            if !path.is_empty() {
                let _ = self.save_current_as(path);
            }
        } else if cmd == "w" || cmd == "write" {
            let _ = self.save_current();
        } else if cmd == "q" || cmd == "quit" {
            self.quit = true;
        } else if cmd == "wq" {
            let _ = self.save_current();
            self.quit = true;
        } else if cmd.starts_with("e ") || cmd.starts_with("edit ") {
            let path = cmd.split_at(2).1.trim();
            if !path.is_empty() {
                self.open_file(path);
            }
        } else if cmd.starts_with("cd ") {
            let path = cmd.split_at(3).1.trim();
            if !path.is_empty() {
                if let Ok(_) = std::env::set_current_dir(path) {
                    self.filetree = FileTree::new(Path::new(path));
                }
            }
        }
    }

    fn handle_key(&mut self, ke: &KeyEvent) {
        if self.filetree_focused {
            match ke.code {
                KeyCode::Up | KeyCode::Char('k') => self.filetree.select_prev(),
                KeyCode::Down | KeyCode::Char('j') => self.filetree.select_next(),
                KeyCode::Enter | KeyCode::Char('l') => {
                    let path = self.filetree.selected_path();
                    if let Some(p) = path {
                        if p.is_dir() {
                            self.filetree.toggle_expand();
                        } else {
                            if let Some(s) = p.to_str() {
                                self.open_file(s);
                            }
                            self.filetree_focused = false;
                        }
                    }
                }
                KeyCode::Char('h') => {
                    self.filetree.toggle_expand();
                }
                KeyCode::Tab => {
                    self.filetree_focused = false;
                }
                _ => {}
            }
            return;
        }

        match ke.code {
            KeyCode::Char('c') if ke.modifiers == KeyModifiers::CONTROL => {
                let buf = self.active_buf();
                if buf.selection().is_none() && !buf.cursors().is_empty() {
                    buf.clear_extra_cursors();
                    return;
                }
            }
            KeyCode::Char('q') if ke.modifiers == KeyModifiers::CONTROL => {
                self.quit = true;
            }
            KeyCode::Char('s') if ke.modifiers == KeyModifiers::CONTROL => {
                let _ = self.save_current();
            }
            KeyCode::Char('o') if ke.modifiers == KeyModifiers::CONTROL => {
                self.filetree_focused = true;
            }
            KeyCode::Char('f') if ke.modifiers == KeyModifiers::CONTROL => {
                self.search.toggle();
                if self.search.active {
                    self.mode = Mode::Search;
                }
            }
            KeyCode::Char('n') if ke.modifiers == KeyModifiers::CONTROL => {
                if ke.modifiers.contains(KeyModifiers::SHIFT) {
                    self.prev_tab();
                } else {
                    self.buffers.push(TextBuffer::new());
                    self.active_buffer = self.buffers.len() - 1;
                }
            }
            KeyCode::Tab if ke.modifiers == KeyModifiers::CONTROL => {
                self.next_tab();
            }
            KeyCode::BackTab if ke.modifiers == KeyModifiers::CONTROL => {
                self.prev_tab();
            }
            KeyCode::Char('w') if ke.modifiers == KeyModifiers::CONTROL => {
                self.close_current_tab();
            }
            KeyCode::Char(':') => {
                self.mode = Mode::Command;
                self.command_buffer.clear();
                self.command_buffer.push(':');
            }
            KeyCode::Esc => {
                let buf = self.active_buf();
                buf.clear_selection();
            }
            KeyCode::Up => {
                let buf = self.active_buf();
                if ke.modifiers.contains(KeyModifiers::SHIFT) {
                    if buf.selection().is_none() {
                        buf.begin_selection();
                    }
                    buf.move_up();
                    buf.update_selection();
                } else {
                    buf.clear_selection();
                    buf.move_up();
                }
            }
            KeyCode::Down => {
                let buf = self.active_buf();
                if ke.modifiers.contains(KeyModifiers::SHIFT) {
                    if buf.selection().is_none() {
                        buf.begin_selection();
                    }
                    buf.move_down();
                    buf.update_selection();
                } else {
                    buf.clear_selection();
                    buf.move_down();
                }
            }
            KeyCode::Left => {
                let buf = self.active_buf();
                if ke.modifiers.contains(KeyModifiers::SHIFT) {
                    if buf.selection().is_none() {
                        buf.begin_selection();
                    }
                    buf.move_left();
                    buf.update_selection();
                } else {
                    buf.clear_selection();
                    buf.move_left();
                }
            }
            KeyCode::Right => {
                let buf = self.active_buf();
                if ke.modifiers.contains(KeyModifiers::SHIFT) {
                    if buf.selection().is_none() {
                        buf.begin_selection();
                    }
                    buf.move_right();
                    buf.update_selection();
                } else {
                    buf.clear_selection();
                    buf.move_right();
                }
            }
            KeyCode::Home => {
                let buf = self.active_buf();
                buf.clear_selection();
                buf.move_home();
            }
            KeyCode::End => {
                let buf = self.active_buf();
                buf.clear_selection();
                buf.move_end();
            }
            KeyCode::PageUp => {
                let buf = self.active_buf();
                buf.clear_selection();
                let h = 20;
                buf.move_page_up(h);
            }
            KeyCode::PageDown => {
                let buf = self.active_buf();
                buf.clear_selection();
                let h = 20;
                buf.move_page_down(h);
            }
            KeyCode::Backspace => {
                self.active_buf().delete_backward();
            }
            KeyCode::Delete => {
                self.active_buf().delete_forward();
            }
            KeyCode::Enter => {
                let buf = self.active_buf();
                buf.insert_char('\n');
            }
            KeyCode::Tab => {
                let buf = self.active_buf();
                buf.insert_str("    ");
            }
            KeyCode::Char(ch) => {
                if ke.modifiers == KeyModifiers::ALT && ch == 'j' {
                    self.active_buf().add_cursor_alt_down();
                } else if ke.modifiers == KeyModifiers::ALT && ch == 'k' {
                    self.active_buf().add_cursor_alt_up();
                } else if ke.modifiers.is_empty() || ke.modifiers == KeyModifiers::SHIFT {
                    let buf = self.active_buf();
                    buf.clear_selection();
                    buf.insert_char(ch);
                }
            }
            _ => {}
        }
    }

    pub fn render(&mut self, f: &mut Frame) {
        let area = f.area();
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(area);

        let tab_area = layout[0];
        let main_area = layout[1];
        let status_area = layout[2];

        self.render_tabs(f, tab_area);

        let main_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(self.tree_width),
                Constraint::Min(1),
            ])
            .split(main_area);

        let tree_area = main_layout[0];
        let editor_area = main_layout[1];

        self.render_filetree(f, tree_area);
        self.render_editor(f, editor_area);
        self.render_status(f, status_area);

        if self.search.active {
            self.render_search_overlay(f, area);
        }
        if let Mode::Command = self.mode {
            self.render_command_bar(f, area);
        }
    }

    fn render_tabs(&self, f: &mut Frame, area: Rect) {
        if area.width < 3 || area.height == 0 {
            return;
        }
        let mut spans = Vec::new();
        for (i, buf) in self.buffers.iter().enumerate() {
            let is_active = i == self.active_buffer;
            let name = if buf.is_dirty() {
                format!(" {} * ", buf.filename())
            } else {
                format!(" {} ", buf.filename())
            };
            let style = if is_active {
                Style::default()
                    .fg(theme::FG_BRIGHT)
                    .bg(theme::TAB_ACTIVE_BG)
            } else {
                Style::default()
                    .fg(theme::FG_DIM)
                    .bg(theme::TAB_INACTIVE_BG)
            };
            spans.push(Span::styled(name, style));
            spans.push(Span::raw(" "));
        }
        let line = Line::from(spans);
        let p = Paragraph::new(line).style(Style::default().bg(theme::BG_DARK));
        f.render_widget(p, area);
    }

    fn render_filetree(&mut self, f: &mut Frame, area: Rect) {
        if area.width < 3 || area.height == 0 {
            return;
        }
        let (list, state) = self.filetree.render(area);
        let style = if self.filetree_focused {
            Block::default()
                .borders(Borders::RIGHT)
                .style(Style::default().bg(theme::BG))
        } else {
            Block::default()
                .borders(Borders::RIGHT)
                .style(Style::default().bg(theme::BG_DARK))
        };
        let list = list.block(style);
        f.render_stateful_widget(list, area, state);
    }

    fn render_editor(&mut self, f: &mut Frame, area: Rect) {
        if area.width < 5 || area.height == 0 {
            return;
        }
        let buf = &self.buffers[self.active_buffer];
        let search = &self.search;
        let p = self.editor.render(buf, area, search);
        f.render_widget(p, area);
    }

    fn render_status(&self, f: &mut Frame, area: Rect) {
        if area.width < 5 || area.height == 0 {
            return;
        }
        let buf = self.active_buf_ref();
        let mode_str = if self.filetree_focused {
            "TREE"
        } else {
            match self.mode {
                Mode::Normal => "INSERT",
                Mode::Command => "CMD",
                Mode::Search => "SEARCH",
            }
        };
        let p = StatusBar::render(buf, mode_str, area);
        f.render_widget(p, area);
    }

    fn render_search_overlay(&self, f: &mut Frame, area: Rect) {
        let popup_area = Rect {
            x: area.width.saturating_sub(40).max(0),
            y: area.height.saturating_sub(2),
            width: 40.min(area.width),
            height: 3,
        };
        if popup_area.width < 10 {
            return;
        }
        f.render_widget(Clear, popup_area);
        let p = self.search.render_search_bar(popup_area);
        f.render_widget(p, popup_area);
    }

    fn render_command_bar(&self, f: &mut Frame, area: Rect) {
        let cmd = &self.command_buffer;
        let popup_area = Rect {
            x: 0,
            y: area.height.saturating_sub(2),
            width: area.width.min(60),
            height: 3,
        };
        if popup_area.width < 5 {
            return;
        }
        f.render_widget(Clear, popup_area);
        let p = Paragraph::new(cmd.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Cmd")
                    .style(Style::default()),
            );
        f.render_widget(p, popup_area);
    }
}

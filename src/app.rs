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
use crate::term_panel::TerminalPanel;
use crate::cmd_palette::CmdPalette;
use crate::theme;
use crate::ai_project::{self, ProjectScaffold};

pub enum Mode {
    Normal,
    Command,
    Search,
    ProjectInput,
    CmdPalette,
}

pub enum Action {
    None,
    Quit,
}

pub struct App {
    pub buffers: Vec<TextBuffer>,
    pub active_buffer: usize,
    pub filetree: FileTree,
    pub search: SearchState,
    pub editor: EditorRenderer,
    pub term_panel: TerminalPanel,
    pub cmd_palette: CmdPalette,
    pub mode: Mode,
    pub command_buffer: String,
    pub quit: bool,
    pub filetree_focused: bool,
    pub tree_width: u16,
    pub action: Action,
    pub terminal_height: u16,

    pub proj_field: usize,
    pub proj_name: String,
    pub proj_lang: String,
    pub proj_desc: String,
    pub proj_status: String,
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
            term_panel: TerminalPanel::new(),
            cmd_palette: CmdPalette::new(),
            mode: Mode::Normal,
            command_buffer: String::new(),
            quit: false,
            filetree_focused: false,
            tree_width: 25,
            action: Action::None,
            terminal_height: 10,

            proj_field: 0,
            proj_name: String::new(),
            proj_lang: String::new(),
            proj_desc: String::new(),
            proj_status: String::new(),
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
        self.action = Action::None;
        match self.mode {
            Mode::Search => self.handle_search_event(&evt),
            Mode::Command => self.handle_command_event(&evt),
            Mode::ProjectInput => self.handle_project_event(&evt),
            Mode::CmdPalette => self.handle_cmd_palette_event(&evt),
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

    fn handle_cmd_palette_event(&mut self, evt: &Event) {
        match evt {
            Event::Key(ke) => match ke.code {
                KeyCode::Esc => {
                    self.cmd_palette.toggle();
                    self.mode = Mode::Normal;
                }
                KeyCode::Enter => {
                    self.execute_cmd_palette();
                }
                KeyCode::Up => {
                    self.cmd_palette.select_prev();
                }
                KeyCode::Down => {
                    self.cmd_palette.select_next();
                }
                KeyCode::Tab => {
                    self.cmd_palette.select_next();
                }
                KeyCode::BackTab => {
                    self.cmd_palette.select_prev();
                }
                KeyCode::Backspace => {
                    self.cmd_palette.pop_char();
                }
                KeyCode::Char(ch) => {
                    self.cmd_palette.push_char(ch);
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn execute_cmd_palette(&mut self) {
        let cmd = self.cmd_palette.selected_cmd().unwrap_or("").to_string();
        self.cmd_palette.toggle();
        self.mode = Mode::Normal;

        match cmd.as_str() {
            "terminal" => {
                if self.term_panel.visible {
                    self.term_panel.stop();
                } else {
                    self.term_panel.start();
                }
            }
            "wsl" => {
                self.term_panel.start_wsl();
            }
            "save" => {
                let _ = self.save_current();
            }
            "save as" => {
                self.mode = Mode::Command;
                self.command_buffer = String::from(":w ");
            }
            "reload" => {
                let _ = self.active_buf().reload();
            }
            "open" => {
                self.filetree_focused = true;
            }
            "new file" => {
                self.buffers.push(TextBuffer::new());
                self.active_buffer = self.buffers.len() - 1;
            }
            "close tab" => {
                self.close_current_tab();
            }
            "close all" => {
                self.buffers.clear();
                self.buffers.push(TextBuffer::new());
                self.active_buffer = 0;
            }
            "search" => {
                self.search.toggle();
                if self.search.active {
                    self.mode = Mode::Search;
                }
            }
            "replace" => {
                self.mode = Mode::Command;
                self.command_buffer = String::from(":%s/");
            }
            "select all" => {
                let buf = self.active_buf();
                buf.clear_extra_cursors();
                buf.select_all();
            }
            "go to line" => {
                self.mode = Mode::Command;
                self.command_buffer = String::from(":goto ");
            }
            "delete line" => {
                let buf = self.active_buf();
                buf.clear_selection();
                buf.delete_line();
            }
            "duplicate line" => {
                let buf = self.active_buf();
                buf.clear_selection();
                buf.duplicate_line();
            }
            "project" => {
                self.start_project_creation();
            }
            "help" => {
                self.show_help();
            }
            "quit" => {
                self.quit = true;
            }
            _ => {}
        }
    }

    fn show_help(&self) {
    }

    fn execute_command(&mut self) {
        let raw = self.command_buffer.trim().to_string();
        self.command_buffer.clear();
        self.mode = Mode::Normal;

        let cmd = raw.strip_prefix(':').unwrap_or(&raw).trim().to_string();

        if cmd == "w" || cmd == "write" {
            let _ = self.save_current();
        } else if cmd.starts_with("w ") {
            let path = cmd[2..].trim();
            if !path.is_empty() {
                let _ = self.save_current_as(path);
            }
        } else if cmd.starts_with("write ") {
            let path = cmd[6..].trim();
            if !path.is_empty() {
                let _ = self.save_current_as(path);
            }
        } else if cmd == "q" || cmd == "quit" {
            self.quit = true;
        } else if cmd == "wq" {
            let _ = self.save_current();
            self.quit = true;
        } else if cmd.starts_with("e ") {
            let path = cmd[2..].trim();
            if !path.is_empty() {
                self.open_file(path);
            }
        } else if cmd.starts_with("edit ") {
            let path = cmd[5..].trim();
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
        } else if cmd == "newproject" || cmd == "np" {
            self.start_project_creation();
        } else if cmd.starts_with("goto ") {
            if let Ok(n) = cmd[5..].trim().parse::<usize>() {
                if n >= 1 {
                    let line = (n - 1).min(self.active_buf().line_count() - 1);
                    self.active_buf().set_cursor(crate::buffer::Cursor::new(line, 0));
                }
            }
        } else if cmd.starts_with("%s/") {
            let rest = &cmd[3..];
            let parts: Vec<&str> = rest.splitn(3, '/').collect();
            if parts.len() == 3 && !parts[0].is_empty() {
                let to = parts[1].replace("\\/", "/");
                self.active_buf().replace_all(parts[0], &to);
            }
        } else if cmd.starts_with("s/") {
            let rest = &cmd[2..];
            let parts: Vec<&str> = rest.splitn(3, '/').collect();
            if parts.len() == 3 && !parts[0].is_empty() {
                let to = parts[1].replace("\\/", "/");
                self.active_buf().replace_all(parts[0], &to);
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

        if self.term_panel.visible && self.term_panel.focused {
            if ke.modifiers == KeyModifiers::CONTROL {
                let byte = match ke.code {
                    KeyCode::Char('c') => Some(b"\x03"),
                    KeyCode::Char('d') => Some(b"\x04"),
                    KeyCode::Char('l') => Some(b"\x0c"),
                    KeyCode::Char('z') => Some(b"\x1a"),
                    _ => None,
                };
                if let Some(data) = byte {
                    self.term_panel.write(data);
                }
                return;
            }
            match ke.code {
                KeyCode::Esc => self.term_panel.focused = false,
                KeyCode::PageUp => self.term_panel.scroll_up(),
                KeyCode::PageDown => self.term_panel.scroll_down(),
                KeyCode::Char(ch) => {
                    let mut buf = [0u8; 4];
                    let s = ch.encode_utf8(&mut buf);
                    self.term_panel.write(s.as_bytes());
                }
                KeyCode::Enter => self.term_panel.write(b"\r\n"),
                KeyCode::Backspace => self.term_panel.write(b"\x08"),
                KeyCode::Tab => self.term_panel.write(b"\t"),
                KeyCode::Up => self.term_panel.write(b"\x1b[A"),
                KeyCode::Down => self.term_panel.write(b"\x1b[B"),
                KeyCode::Right => self.term_panel.write(b"\x1b[C"),
                KeyCode::Left => self.term_panel.write(b"\x1b[D"),
                KeyCode::Home => self.term_panel.write(b"\x1b[H"),
                KeyCode::End => self.term_panel.write(b"\x1b[F"),
                KeyCode::Delete => self.term_panel.write(b"\x1b[3~"),
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
            KeyCode::Char('/') if ke.modifiers == KeyModifiers::CONTROL => {
                self.cmd_palette.toggle();
                if self.cmd_palette.active {
                    self.mode = Mode::CmdPalette;
                } else {
                    self.mode = Mode::Normal;
                }
            }
            KeyCode::Char('t') if ke.modifiers == KeyModifiers::CONTROL => {
                if self.term_panel.visible {
                    if self.term_panel.focused {
                        self.term_panel.focused = false;
                        self.term_panel.stop();
                    } else {
                        self.term_panel.focused = true;
                    }
                } else {
                    self.term_panel.start();
                }
            }
            KeyCode::Char('`') if ke.modifiers == KeyModifiers::CONTROL => {
                if self.term_panel.visible {
                    self.term_panel.focused = !self.term_panel.focused;
                }
            }
            KeyCode::Char('p') if ke.modifiers == KeyModifiers::CONTROL => {
                self.start_project_creation();
            }
            KeyCode::Char('n') if ke.modifiers == KeyModifiers::CONTROL => {
                self.buffers.push(TextBuffer::new());
                self.active_buffer = self.buffers.len() - 1;
            }
            KeyCode::Char('N') if ke.modifiers == KeyModifiers::CONTROL => {
                self.prev_tab();
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
            KeyCode::F(1) => {
                if self.term_panel.visible {
                    self.term_panel.focused = true;
                }
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

    fn start_project_creation(&mut self) {
        self.mode = Mode::ProjectInput;
        self.proj_name.clear();
        self.proj_lang = String::from("rust");
        self.proj_desc.clear();
        self.proj_field = 0;
        self.proj_status.clear();
    }

    fn handle_project_event(&mut self, evt: &Event) {
        match evt {
            Event::Key(ke) => match ke.code {
                KeyCode::Esc => {
                    self.mode = Mode::Normal;
                }
                KeyCode::Tab => {
                    self.proj_field = (self.proj_field + 1) % 3;
                }
                KeyCode::BackTab => {
                    self.proj_field = if self.proj_field == 0 { 2 } else { self.proj_field - 1 };
                }
                KeyCode::Enter => {
                    self.execute_project_creation();
                }
                KeyCode::Backspace => {
                    match self.proj_field {
                        0 => { self.proj_name.pop(); }
                        1 => { self.proj_lang.pop(); }
                        _ => { self.proj_desc.pop(); }
                    }
                }
                KeyCode::Char(ch) => {
                    match self.proj_field {
                        0 => self.proj_name.push(ch),
                        1 => self.proj_lang.push(ch),
                        _ => self.proj_desc.push(ch),
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn execute_project_creation(&mut self) {
        if self.proj_name.is_empty() {
            self.proj_status = String::from("Error: Project name is required");
            return;
        }
        let lang = if self.proj_lang.is_empty() {
            String::from("rust")
        } else {
            self.proj_lang.clone()
        };
        let scaffold = ProjectScaffold {
            name: self.proj_name.clone(),
            language: lang,
            description: self.proj_desc.clone(),
        };
        self.proj_status = String::from("Creating project...");
        match ai_project::create_project(&scaffold) {
            Ok(()) => {
                self.proj_status = format!("Project '{}' created!", self.proj_name);
                let cwd = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
                self.filetree = FileTree::new(&cwd);
                self.mode = Mode::Normal;
            }
            Err(e) => {
                self.proj_status = format!("Error: {e}");
            }
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

        if self.term_panel.visible {
            let main_vert = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),
                    Constraint::Length(self.terminal_height.min(area.height.saturating_sub(5))),
                ])
                .split(main_area);

            let editor_zone = main_vert[0];
            let term_area = main_vert[1];

            let main_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(self.tree_width),
                    Constraint::Min(1),
                ])
                .split(editor_zone);

            let tree_area = main_layout[0];
            let editor_area = main_layout[1];

            self.render_filetree(f, tree_area);
            self.render_editor(f, editor_area);
            f.render_widget(self.term_panel.render(term_area), term_area);
        } else {
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
        }
        self.render_status(f, status_area);

        if self.search.active {
            self.render_search_overlay(f, area);
        }
        if let Mode::Command = self.mode {
            self.render_command_bar(f, area);
        }
        if let Mode::ProjectInput = self.mode {
            self.render_project_form(f, area);
        }
        if let Mode::CmdPalette = self.mode {
            self.cmd_palette.render(area, f);
        }
    }

    fn render_project_form(&self, f: &mut Frame, area: Rect) {
        let form_w = 55.min(area.width.saturating_sub(4));
        let form_h = 14.min(area.height.saturating_sub(4));
        let popup = Rect {
            x: (area.width - form_w) / 2,
            y: (area.height - form_h) / 2,
            width: form_w,
            height: form_h,
        };
        if popup.width < 20 || popup.height < 5 {
            return;
        }

        f.render_widget(Clear, popup);

        let fields = [
            ("Project Name", self.proj_name.as_str()),
            ("Language (rust/python/js/ts/go/c/cpp/html)", self.proj_lang.as_str()),
            ("Description", self.proj_desc.as_str()),
        ];

        let mut lines = vec![
            Line::from(Span::styled(
                " New Project (Tab to switch, Enter to create, Esc to cancel)",
                Style::default().fg(theme::ACCENT),
            )),
            Line::from(Span::raw("")),
        ];

        for (i, (label, val)) in fields.iter().enumerate() {
            let is_active = i == self.proj_field;
            let prefix = if is_active { "> " } else { "  " };
            let label_style = if is_active {
                Style::default().fg(theme::ACCENT).bg(theme::BG_LIGHT)
            } else {
                Style::default().fg(theme::FG_DIM)
            };
            let val_style = if val.is_empty() {
                Style::default().fg(theme::FG_DIM)
            } else {
                Style::default().fg(theme::FG_BRIGHT)
            };
            lines.push(Line::from(vec![
                Span::styled(format!("{}{}: ", prefix, label), label_style),
                Span::styled(val.to_string(), val_style),
            ]));
        }

        if !self.proj_status.is_empty() {
            lines.push(Line::from(Span::raw("")));
            let status_color = if self.proj_status.starts_with("Error") {
                theme::ACCENT_RED
            } else if self.proj_status == "Creating project..." {
                theme::ACCENT_ORANGE
            } else {
                theme::ACCENT_GREEN
            };
            lines.push(Line::from(Span::styled(
                &self.proj_status,
                Style::default().fg(status_color),
            )));
        }

        let p = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Project Creator").style(Style::default().bg(theme::BG)));
        f.render_widget(p, popup);
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
        let (mut list, state) = self.filetree.render(area);
        let focused = self.filetree_focused;
        let title = if focused {
            " Files (↑↓ j/k · Enter open · h collapse · Tab exit) "
        } else {
            " Files (Ctrl+O focus) "
        };
        let style = if focused {
            Block::default()
                .title(title)
                .borders(Borders::RIGHT)
                .style(Style::default().bg(theme::BG))
        } else {
            Block::default()
                .title(title)
                .borders(Borders::RIGHT)
                .style(Style::default().bg(theme::BG_DARK))
        };
        list = list.block(style);
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

        if let Mode::Normal = self.mode {
            if !self.filetree_focused && !(self.term_panel.visible && self.term_panel.focused) {
                let cursor = buf.cursor();
                let gutter_w = 6;
                let screen_x = area.x + gutter_w + cursor.col as u16;
                let screen_y = area.y + cursor.line as u16;
                if screen_x < area.x + area.width && screen_y < area.y + area.height {
                    f.set_cursor_position((screen_x, screen_y));
                }
            }
        }
    }

    fn render_status(&self, f: &mut Frame, area: Rect) {
        if area.width < 5 || area.height == 0 {
            return;
        }
        let buf = self.active_buf_ref();
        let mode_str = if self.term_panel.visible && self.term_panel.focused {
            "TERM"
        } else if self.filetree_focused {
            "TREE"
        } else {
            match self.mode {
                Mode::Normal => "INSERT",
                Mode::Command => "CMD",
                Mode::Search => "SEARCH",
                Mode::ProjectInput => "PROJECT",
                Mode::CmdPalette => "PALETTE",
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

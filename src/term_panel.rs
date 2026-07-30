use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use crate::theme;

const MAX_LINES: usize = 5000;

struct TerminalBuffer {
    lines: Vec<String>,
    raw_line: String,
}

impl TerminalBuffer {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            raw_line: String::new(),
        }
    }

    fn push_char(&mut self, ch: char) {
        if ch == '\r' { return; }
        if ch == '\n' {
            let line = std::mem::take(&mut self.raw_line);
            self.lines.push(line);
            if self.lines.len() > MAX_LINES { self.lines.remove(0); }
            return;
        }
        if ch == '\x08' || ch == '\x7f' {
            self.raw_line.pop();
            return;
        }
        self.raw_line.push(ch);
    }

    fn flush_line(&mut self) {
        if !self.raw_line.is_empty() || self.lines.is_empty() {
            let line = std::mem::take(&mut self.raw_line);
            self.lines.push(line);
            if self.lines.len() > MAX_LINES { self.lines.remove(0); }
        }
    }

    fn content(&self, scroll: usize, height: usize) -> Vec<String> {
        let total = self.lines.len();
        let end = total.saturating_sub(scroll);
        let start = end.saturating_sub(height);
        self.lines[start..end].iter().map(|l| strip_ansi(l)).collect()
    }

    fn set_lines(&mut self, text: &str) {
        self.lines.clear();
        for line in text.lines() {
            self.lines.push(line.to_string());
        }
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
    }
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.next() == Some('[') {
            while let Some(n) = chars.next() {
                if n == 'm' || n == 'H' || n == 'J' || n == 'K' || n == 'A' || n == 'B'
                    || n == 'C' || n == 'D' || n == 's' || n == 'u' || n == 'h' || n == 'l'
                { break; }
                if n == '?' { continue; }
                if n.is_ascii_alphabetic() { break; }
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub struct TerminalPanel {
    buffer: Arc<Mutex<TerminalBuffer>>,
    writer: Option<Box<dyn Write + Send>>,
    child_killer: Option<Box<dyn portable_pty::ChildKiller + Send>>,
    pub visible: bool,
    pub focused: bool,
    scroll: usize,
    pub shell_type: String,
    error_msg: String,
}

impl TerminalPanel {
    pub fn new() -> Self {
        let buf = Arc::new(Mutex::new(TerminalBuffer::new()));
        buf.lock().unwrap().set_lines("Press Ctrl+T to open terminal. Press F1 to focus it.");
        Self {
            buffer: buf,
            writer: None,
            child_killer: None,
            visible: false,
            focused: false,
            scroll: 0,
            shell_type: String::from("cmd"),
            error_msg: String::new(),
        }
    }

    fn read_loop(reader: Box<dyn Read + Send>, buf: Arc<Mutex<TerminalBuffer>>) {
        let mut reader = reader;
        let mut tmp = [0u8; 4096];
        loop {
            match reader.read(&mut tmp) {
                Ok(0) => break,
                Ok(n) => {
                    let mut lock = buf.lock().unwrap();
                    for &b in &tmp[..n] { lock.push_char(b as char); }
                }
                Err(_) => break,
            }
        }
    }

    fn spawn_pty(shell: &str, rows: u16, cols: u16) -> Result<
        (Box<dyn portable_pty::ChildKiller + Send>, Box<dyn Write + Send>, Box<dyn Read + Send>),
        String
    > {
        let pty_system = portable_pty::native_pty_system();
        let size = portable_pty::PtySize { rows, cols, pixel_width: 0, pixel_height: 0 };
        let pair = pty_system.openpty(size).map_err(|e| format!("PTY error: {e}"))?;
        let cmd = portable_pty::CommandBuilder::new(shell);
        let child = pair.slave.spawn_command(cmd).map_err(|e| format!("Spawn error: {e}"))?;
        let killer = child.clone_killer();
        let reader = pair.master.try_clone_reader().map_err(|e| format!("Reader error: {e}"))?;
        let writer = pair.master.take_writer().map_err(|e| format!("Writer error: {e}"))?;
        Ok((killer, writer, reader))
    }

    pub fn start(&mut self) {
        self.start_shell("cmd.exe");
    }

    pub fn start_wsl(&mut self) {
        self.start_shell("wsl.exe");
    }

    pub fn start_shell(&mut self, shell: &str) {
        self.stop();
        self.error_msg.clear();

        match Self::spawn_pty(shell, self.rows(), self.cols()) {
            Ok((killer, mut writer, reader)) => {
                let buf = self.buffer.clone();
                let msg = format!("Starting {}\r\n", shell);
                buf.lock().unwrap().set_lines(&msg);
                let _ = writer.write_all(b"\r\n");
                let _ = writer.flush();
                thread::spawn(move || Self::read_loop(reader, buf));
                self.child_killer = Some(killer);
                self.writer = Some(writer);
                self.visible = true;
                self.focused = true;
                self.scroll = 0;
                self.shell_type = shell.to_string();
            }
            Err(e) => {
                self.error_msg = format!("Terminal error: {e}");
                self.visible = true;
                self.focused = false;
                {
                    let mut buf = self.buffer.lock().unwrap();
                    buf.set_lines(&format!(
                        "Failed to start terminal: {e}\n\n\
                         Press Ctrl+T to close this panel."
                    ));
                }
            }
        }
    }

    pub fn stop(&mut self) {
        if let Some(mut killer) = self.child_killer.take() {
            let _ = killer.kill();
        }
        self.writer = None;
        self.visible = false;
        self.focused = false;
        self.buffer.lock().unwrap().flush_line();
    }

    fn rows(&self) -> u16 { 20 }
    fn cols(&self) -> u16 { 80 }

    pub fn write(&mut self, data: &[u8]) {
        if let Some(ref mut w) = self.writer {
            let _ = w.write_all(data);
            let _ = w.flush();
        }
    }

    pub fn write_str(&mut self, s: &str) { self.write(s.as_bytes()); }

    pub fn scroll_up(&mut self) {
        let total = self.buffer.lock().unwrap().lines.len();
        if self.scroll < total.saturating_sub(1) { self.scroll += 1; }
    }

    pub fn scroll_down(&mut self) {
        if self.scroll > 0 { self.scroll -= 1; }
    }

    pub fn render(&self, area: Rect) -> Paragraph {
        let height = area.height.saturating_sub(2) as usize;
        let buf = self.buffer.lock().unwrap();
        let content = buf.content(self.scroll, height.max(1));

        let lines: Vec<Line> = if content.iter().all(|l| l.is_empty() || l.starts_with("Starting ")) {
            vec![Line::from(Span::styled("Terminal ready — waiting for shell output...", Style::default().fg(theme::FG_DIM)))]
        } else {
            content.into_iter().map(|l| Line::from(Span::raw(l))).collect()
        };

        let shell = &self.shell_type;
        let title = if self.focused {
            format!(" Terminal [{}] (focused — Esc to unfocus) ", shell)
        } else {
            format!(" Terminal [{}] ", shell)
        };
        let border_style = if self.focused {
            Style::default().fg(theme::ACCENT)
        } else {
            Style::default().fg(theme::FG_DIM)
        };

        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(title).border_style(border_style).style(Style::default().bg(theme::BG_DARK)))
            .wrap(Wrap { trim: false })
    }
}

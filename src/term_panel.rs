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
        if ch == '\r' {
            return;
        }
        if ch == '\n' {
            let line = std::mem::take(&mut self.raw_line);
            self.lines.push(line);
            if self.lines.len() > MAX_LINES {
                self.lines.remove(0);
            }
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
            if self.lines.len() > MAX_LINES {
                self.lines.remove(0);
            }
        }
    }

    fn content(&self, scroll: usize, height: usize) -> Vec<String> {
        let total = self.lines.len();
        let end = total.saturating_sub(scroll);
        let start = end.saturating_sub(height);
        self.lines[start..end]
            .iter()
            .map(|l| strip_ansi(l))
            .collect()
    }
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.next() == Some('[') {
                while let Some(n) = chars.next() {
                    if n == 'm' || n == 'H' || n == 'J' || n == 'K' || n == 'A' || n == 'B'
                        || n == 'C' || n == 'D' || n == 's' || n == 'u' || n == 'h' || n == 'l'
                    {
                        break;
                    }
                    if n == '?' { continue; }
                    if n.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub struct TerminalPanel {
    buffer: Arc<Mutex<TerminalBuffer>>,
    child: Option<Box<dyn portable_pty::Child + Send>>,
    writer: Option<Box<dyn Write + Send>>,
    pub visible: bool,
    pub focused: bool,
    scroll: usize,
    rows: u16,
    cols: u16,
}

impl TerminalPanel {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(TerminalBuffer::new())),
            child: None,
            writer: None,
            visible: false,
            focused: false,
            scroll: 0,
            rows: 20,
            cols: 80,
        }
    }

    pub fn start(&mut self) {
        if self.visible {
            return;
        }

        let pty_system = portable_pty::native_pty_system();
        let size = portable_pty::PtySize {
            rows: self.rows,
            cols: self.cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        match pty_system.openpty(size) {
            Ok(pair) => {
                let shell = if cfg!(windows) { "cmd.exe" } else { "/bin/sh" };
                let cmd = portable_pty::CommandBuilder::new(shell);
                let child = pair.slave.spawn_command(cmd);

                match child {
                    Ok(child) => {
                        self.child = Some(child);
                        let reader = pair.master.try_clone_reader();
                        let writer = pair.master.take_writer();

                        if let Ok(reader) = reader {
                            let buf = self.buffer.clone();
                            thread::spawn(move || {
                                let mut reader = reader;
                                let mut tmp = [0u8; 4096];
                                loop {
                                    match reader.read(&mut tmp) {
                                        Ok(0) => break,
                                        Ok(n) => {
                                            let mut buf_lock = buf.lock().unwrap();
                                            for &b in &tmp[..n] {
                                                buf_lock.push_char(b as char);
                                            }
                                        }
                                        Err(_) => break,
                                    }
                                }
                            });
                        }

                        self.writer = writer.ok();
                        self.visible = true;
                        self.scroll = 0;
                    }
                    Err(_) => {}
                }
            }
            Err(_) => {}
        }
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
        }
        self.writer = None;
        self.visible = false;
        self.buffer.lock().unwrap().flush_line();
    }

    pub fn write(&mut self, data: &[u8]) {
        if let Some(ref mut w) = self.writer {
            let _ = w.write_all(data);
            let _ = w.flush();
        }
    }

    pub fn write_str(&mut self, s: &str) {
        self.write(s.as_bytes());
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.rows = rows;
        self.cols = cols;
    }

    pub fn scroll_up(&mut self) {
        let total = self.buffer.lock().unwrap().lines.len();
        if self.scroll < total.saturating_sub(1) {
            self.scroll += 1;
        }
    }

    pub fn scroll_down(&mut self) {
        if self.scroll > 0 {
            self.scroll -= 1;
        }
    }

    pub fn scroll_bottom(&mut self) {
        self.scroll = 0;
    }

    pub fn render(&self, area: Rect) -> Paragraph {
        let height = area.height.saturating_sub(2) as usize;
        let buf = self.buffer.lock().unwrap();
        let content = buf.content(self.scroll, height.max(1));

        let lines: Vec<Line> = if content.is_empty() {
            vec![Line::from(Span::raw("Terminal ready. Type a command..."))]
        } else {
            content.into_iter().map(|l| Line::from(Span::raw(l))).collect()
        };

        let title = if self.focused { " Terminal (focused) " } else { " Terminal " };
        let border_style = if self.focused {
            Style::default().fg(theme::ACCENT)
        } else {
            Style::default().fg(theme::FG_DIM)
        };

        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(border_style)
                    .style(Style::default().bg(theme::BG_DARK)),
            )
            .wrap(Wrap { trim: false })
    }
}

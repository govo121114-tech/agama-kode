use std::io::Read;
use std::sync::{Arc, Mutex};
use std::thread;
use portable_pty::{ChildKiller, CommandBuilder, native_pty_system, PtyPair, PtySize};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use crate::theme;

const MAX_LINES: usize = 5000;

struct TerminalBuffer {
    raw: String,
}

impl TerminalBuffer {
    fn new() -> Self {
        Self { raw: String::new() }
    }

    fn append(&mut self, s: &str) {
        self.raw.push_str(s);
        if self.raw.len() > MAX_LINES * 200 {
            self.raw = self.raw.split_off(self.raw.len() - MAX_LINES * 100);
        }
    }

    fn display_lines(&self, scroll: usize, height: usize) -> Vec<String> {
        let text = process_pty(&self.raw);
        let mut lines: Vec<&str> = text.split('\n').collect();
        if lines.last().map_or(false, |l| l.is_empty()) {
            lines.pop();
        }
        let total = lines.len();
        let end = total.saturating_sub(scroll);
        let start = end.saturating_sub(height);
        lines[start..end].iter().map(|l| l.to_string()).collect()
    }
}

fn process_pty(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\x1b' => {
                match chars.peek() {
                    Some('[') => {
                        chars.next();
                        while let Some(&n) = chars.peek() {
                            if n == '\x1b' { break; }
                            if n.is_ascii_alphabetic() || n == '~' {
                                chars.next();
                                break;
                            }
                            chars.next();
                        }
                    }
                    Some(']') => {
                        chars.next();
                        loop {
                            match chars.next() {
                                Some('\x07') => break,
                                Some('\x1b') => { if chars.next() == Some('\\') { break; } }
                                Some(_) => {}
                                None => break,
                            }
                        }
                    }
                    Some(&'(') | Some(&')') | Some(&'#') | Some(&'%') => { chars.next(); }
                    _ => {}
                }
            }
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    continue;
                }
                if let Some(pos) = out.rfind('\n') {
                    out.truncate(pos + 1);
                } else {
                    out.clear();
                }
            }
            '\n' => out.push('\n'),
            '\t' => out.push('\t'),
            c if c.is_ascii_control() && c != '\n' && c != '\t' => {}
            c => out.push(c),
        }
    }
    out
}

struct ShellProcess {
    writer: Box<dyn Write + Send>,
    _child: Box<dyn ChildKiller>,
    _pair: PtyPair,
}

use std::io::Write;

pub struct TerminalPanel {
    buffer: Arc<Mutex<TerminalBuffer>>,
    shell: Option<ShellProcess>,
    pub visible: bool,
    pub focused: bool,
    scroll: usize,
    pub shell_type: String,
}

impl TerminalPanel {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(TerminalBuffer::new())),
            shell: None,
            visible: false,
            focused: false,
            scroll: 0,
            shell_type: String::from("cmd"),
        }
    }

    pub fn start(&mut self) { self.start_shell("cmd.exe"); }
    pub fn start_wsl(&mut self) { self.start_shell("wsl.exe"); }

    pub fn start_shell(&mut self, shell: &str) {
        self.stop();

        let pty_system = native_pty_system();
        let pair = match pty_system.openpty(PtySize {
            rows: 40,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        }) {
            Ok(p) => p,
            Err(e) => {
                let mut b = self.buffer.lock().unwrap();
                b.raw.clear();
                b.raw.push_str(&format!("PTY init error: {e}"));
                self.visible = true;
                self.focused = false;
                return;
            }
        };

        let mut cmd = CommandBuilder::new(shell);
        if shell == "wsl.exe" {
            cmd.arg("--login");
        }
        let child = match pair.slave.spawn_command(cmd) {
            Ok(c) => c,
            Err(e) => {
                let mut b = self.buffer.lock().unwrap();
                b.raw.clear();
                b.raw.push_str(&format!("Spawn error: {e}"));
                b.raw.push('\n');
                b.raw.push_str("Make sure the shell is installed.");
                self.visible = true;
                self.focused = false;
                return;
            }
        };

        let mut reader = match pair.master.try_clone_reader() {
            Ok(r) => r,
            Err(e) => {
                let mut b = self.buffer.lock().unwrap();
                b.raw.clear();
                b.raw.push_str(&format!("PTY reader error: {e}"));
                self.visible = true;
                self.focused = false;
                return;
            }
        };
        let mut writer = match pair.master.take_writer() {
            Ok(w) => w,
            Err(e) => {
                let mut b = self.buffer.lock().unwrap();
                b.raw.clear();
                b.raw.push_str(&format!("PTY writer error: {e}"));
                self.visible = true;
                self.focused = false;
                return;
            }
        };

        {
            let mut b = self.buffer.lock().unwrap();
            b.raw.clear();
        }

        let buf = self.buffer.clone();
        thread::spawn(move || {
            let mut temp = [0u8; 4096];
            let mut carry = String::new();
            loop {
                let n = match reader.read(&mut temp) {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(_) => break,
                };
                carry.push_str(&String::from_utf8_lossy(&temp[..n]));
                buf.lock().unwrap().append(&carry);
                carry.clear();
            }
        });

        let _ = writeln!(&mut writer as &mut dyn Write, "");
        let _ = writer.flush();

        self.shell = Some(ShellProcess {
            writer,
            _child: child,
            _pair: pair,
        });
        self.visible = true;
        self.focused = true;
        self.scroll = 0;
        self.shell_type = shell.to_string();
    }

    pub fn stop(&mut self) {
        if let Some(mut s) = self.shell.take() {
            let _ = s.writer.flush();
            let _ = s._child.kill();
        }
        self.visible = false;
        self.focused = false;
    }

    pub fn write(&mut self, data: &[u8]) {
        if let Some(ref mut s) = self.shell {
            let _ = s.writer.write_all(data);
            let _ = s.writer.flush();
        }
    }

    pub fn scroll_up(&mut self) {
        let total = {
            let b = self.buffer.lock().unwrap();
            let text = process_pty(&b.raw);
            text.split('\n').filter(|l| !l.is_empty()).count()
        };
        if self.scroll < total.saturating_sub(1) { self.scroll += 1; }
    }

    pub fn scroll_down(&mut self) {
        if self.scroll > 0 { self.scroll -= 1; }
    }

    pub fn render(&self, area: Rect) -> Paragraph {
        let height = area.height.saturating_sub(2) as usize;
        let buf = self.buffer.lock().unwrap();
        let content = buf.display_lines(self.scroll, height.max(1));

        let lines: Vec<Line> = if content.is_empty() {
            vec![Line::from(Span::styled("No output yet.", Style::default().fg(theme::FG_DIM)))]
        } else {
            content.into_iter().map(|l| Line::from(Span::raw(l))).collect()
        };

        let shell = &self.shell_type;
        let title = format!(" Terminal [{}] ", shell);
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

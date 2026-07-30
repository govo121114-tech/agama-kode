use std::io::{BufRead, BufReader, Read, Write};
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
    lines: Vec<String>,
}

impl TerminalBuffer {
    fn new() -> Self {
        Self { lines: Vec::new() }
    }

    fn push(&mut self, line: String) {
        self.lines.push(line);
        if self.lines.len() > MAX_LINES { self.lines.remove(0); }
    }

    fn content(&self, scroll: usize, height: usize) -> Vec<String> {
        let total = self.lines.len();
        let end = total.saturating_sub(scroll);
        let start = end.saturating_sub(height);
        self.lines[start..end].iter().map(|l| strip_ansi(l)).collect()
    }
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.next() == Some('[') {
            while let Some(n) = chars.next() {
                if n.is_ascii_alphabetic() || n == '~' { break; }
            }
        } else {
            out.push(c);
        }
    }
    out
}

struct ShellProcess {
    writer: Box<dyn Write + Send>,
    _child: Box<dyn ChildKiller>,
    _pair: PtyPair,
}

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
        let mut pair = match pty_system.openpty(PtySize {
            rows: 40,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        }) {
            Ok(p) => p,
            Err(e) => {
                let mut b = self.buffer.lock().unwrap();
                b.lines.clear();
                b.push(format!("PTY init error: {e}"));
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
                b.lines.clear();
                b.push(format!("Spawn error: {e}"));
                b.push(String::new());
                b.push("Make sure the shell is installed.".to_string());
                self.visible = true;
                self.focused = false;
                return;
            }
        };

        let reader = match pair.master.try_clone_reader() {
            Ok(r) => r,
            Err(e) => {
                let mut b = self.buffer.lock().unwrap();
                b.lines.clear();
                b.push(format!("PTY reader error: {e}"));
                self.visible = true;
                self.focused = false;
                return;
            }
        };
        let mut writer = match pair.master.take_writer() {
            Ok(w) => w,
            Err(e) => {
                let mut b = self.buffer.lock().unwrap();
                b.lines.clear();
                b.push(format!("PTY writer error: {e}"));
                self.visible = true;
                self.focused = false;
                return;
            }
        };

        {
            let mut b = self.buffer.lock().unwrap();
            b.lines.clear();
            b.push(format!("Terminal [{}] started", shell));
        }

        let buf = self.buffer.clone();
        thread::spawn(move || {
            let mut reader = BufReader::new(reader);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        let s = line.trim_end_matches('\r').trim_end_matches('\n').to_string();
                        buf.lock().unwrap().push(s);
                    }
                    Err(_) => break,
                }
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

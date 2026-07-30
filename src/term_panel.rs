use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
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
}

impl TerminalBuffer {
    fn new() -> Self {
        let mut b = Self { lines: Vec::new() };
        b.push("Terminal ready. Press Ctrl+T to open.".to_string());
        b
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
    child: Child,
    stdin: Box<dyn Write + Send>,
    started: String,
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

        let mut cmd = if shell == "wsl.exe" {
            let mut c = Command::new(shell);
            c.arg("--login");
            c
        } else {
            Command::new(shell)
        };

        let child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        match child {
            Ok(mut child) => {
                let stdin = child.stdin.take().map(|s| Box::new(s) as Box<dyn Write + Send>);

                if let Some(mut stdin) = stdin {
                    let buf = self.buffer.clone();
                    let started = shell.to_string();

                    if let Some(stdout) = child.stdout.take() {
                        let buf2 = buf.clone();
                        thread::spawn(move || {
                            let reader = BufReader::new(stdout);
                            for line in reader.lines() {
                                match line {
                                    Ok(l) => buf2.lock().unwrap().push(l),
                                    Err(_) => break,
                                }
                            }
                        });
                    }
                    if let Some(stderr) = child.stderr.take() {
                        let buf2 = buf.clone();
                        thread::spawn(move || {
                            let reader = BufReader::new(stderr);
                            for line in reader.lines() {
                                match line {
                                    Ok(l) => buf2.lock().unwrap().push(l),
                                    Err(_) => break,
                                }
                            }
                        });
                    }

                    {
                        let mut b = self.buffer.lock().unwrap();
                        b.lines.clear();
                        b.push(shell.to_string());
                        b.push(String::new());
                    }
                    let _ = writeln!(&mut stdin as &mut dyn Write, "");
                    let _ = (&mut stdin as &mut dyn Write).flush();

                    self.shell = Some(ShellProcess { child, stdin, started: started.clone() });
                    self.visible = true;
                    self.focused = true;
                    self.scroll = 0;
                    self.shell_type = started;
                }
            }
            Err(e) => {
                let mut b = self.buffer.lock().unwrap();
                b.lines.clear();
                b.push(format!("Failed to start {}: {}", shell, e));
                b.push(String::new());
                b.push("Make sure the shell is installed.".to_string());
                self.visible = true;
                self.focused = false;
            }
        }
    }

    pub fn stop(&mut self) {
        if let Some(mut s) = self.shell.take() {
            let _ = s.child.kill();
            let _ = s.child.wait();
        }
        self.visible = false;
        self.focused = false;
    }

    pub fn write(&mut self, data: &[u8]) {
        if let Some(ref mut s) = self.shell {
            let _ = s.stdin.write_all(data);
            let _ = s.stdin.flush();
        }
    }

    pub fn write_and_echo(&mut self, data: &[u8]) {
        self.write(data);
        if let Ok(s) = std::str::from_utf8(data) {
            let mut b = self.buffer.lock().unwrap();
            if data == b"\x08" {
                if let Some(last) = b.lines.last_mut() {
                    last.pop();
                }
            } else if data == b"\r\n" || data == b"\n" {
                b.push(String::new());
            } else if data == b"\t" {
                if let Some(last) = b.lines.last_mut() {
                    last.push('\t');
                }
            } else if s.chars().all(|c| c.is_ascii_graphic() || c == ' ') {
                if let Some(last) = b.lines.last_mut() {
                    last.push_str(s);
                }
            }
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

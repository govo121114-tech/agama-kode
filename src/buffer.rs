use std::path::{Path, PathBuf};
use std::fs;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cursor {
    pub line: usize,
    pub col: usize,
}

impl Cursor {
    pub fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

#[derive(Clone, Debug)]
pub struct Selection {
    pub start: Cursor,
    pub end: Cursor,
}

impl Selection {
    pub fn new(start: Cursor, end: Cursor) -> Self {
        Self { start, end }
    }

    pub fn sorted(&self) -> (Cursor, Cursor) {
        if self.start.line < self.end.line
            || (self.start.line == self.end.line && self.start.col < self.end.col)
        {
            (self.start, self.end)
        } else {
            (self.end, self.start)
        }
    }
}

#[derive(Clone, Debug)]
pub struct SearchMatch {
    pub line: usize,
    pub start_col: usize,
    pub end_col: usize,
}

#[derive(Clone)]
pub struct TextBuffer {
    lines: Vec<String>,
    path: Option<PathBuf>,
    filename: String,
    dirty: bool,
    cursor: Cursor,
    saved_cursor: Cursor,
    selection: Option<Selection>,
    cursors: Vec<Cursor>,
    line_ending: String,
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            path: None,
            filename: String::from("Untitled"),
            dirty: false,
            cursor: Cursor::new(0, 0),
            saved_cursor: Cursor::new(0, 0),
            selection: None,
            cursors: Vec::new(),
            line_ending: String::from("\n"),
        }
    }

    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path).map_err(|e| format!("Cannot read file: {e}"))?;
        let line_ending = if content.contains("\r\n") { "\r\n" } else { "\n" };
        let lines: Vec<String> = content
            .split('\n')
            .map(|l| {
                if l.ends_with('\r') {
                    l[..l.len() - 1].to_string()
                } else {
                    l.to_string()
                }
            })
            .collect();
        let lines = if lines.is_empty() {
            vec![String::new()]
        } else {
            lines
        };
        let filename = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| String::from("Unknown"));

        Ok(Self {
            lines,
            path: Some(path.to_path_buf()),
            filename,
            dirty: false,
            cursor: Cursor::new(0, 0),
            saved_cursor: Cursor::new(0, 0),
            selection: None,
            cursors: Vec::new(),
            line_ending: line_ending.to_string(),
        })
    }

    pub fn save(&mut self) -> Result<(), String> {
        let path = self.path.clone().ok_or("No file path set")?;
        let content = self.lines.join(&self.line_ending);
        fs::write(&path, content).map_err(|e| format!("Cannot write file: {e}"))?;
        self.dirty = false;
        Ok(())
    }

    pub fn save_as(&mut self, path: &Path) -> Result<(), String> {
        let filename = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| String::from("Unknown"));
        let content = self.lines.join(&self.line_ending);
        fs::write(path, content).map_err(|e| format!("Cannot write file: {e}"))?;
        self.path = Some(path.to_path_buf());
        self.filename = filename;
        self.dirty = false;
        Ok(())
    }

    pub fn set_path(&mut self, path: &Path) {
        self.path = Some(path.to_path_buf());
        self.filename = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| String::from("Unknown"));
    }

    pub fn lines(&self) -> &Vec<String> { &self.lines }
    pub fn line(&self, idx: usize) -> Option<&str> {
        self.lines.get(idx).map(|s| s.as_str())
    }
    pub fn line_count(&self) -> usize { self.lines.len() }
    pub fn path(&self) -> Option<&PathBuf> { self.path.as_ref() }
    pub fn filename(&self) -> &str { &self.filename }
    pub fn is_dirty(&self) -> bool { self.dirty }
    pub fn cursor(&self) -> Cursor { self.cursor }
    pub fn selection(&self) -> Option<&Selection> { self.selection.as_ref() }
    pub fn cursors(&self) -> &[Cursor] { &self.cursors }

    pub fn line_len(&self, line: usize) -> usize {
        self.lines.get(line).map(|l| l.len()).unwrap_or(0)
    }

    pub fn set_cursor(&mut self, c: Cursor) {
        self.cursor = c;
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    pub fn begin_selection(&mut self) {
        self.selection = Some(Selection::new(self.cursor, self.cursor));
    }

    pub fn update_selection(&mut self) {
        if let Some(ref mut sel) = self.selection {
            sel.end = self.cursor;
        }
    }

    pub fn selected_text(&self) -> Option<String> {
        self.selection.as_ref().map(|sel| {
            let (start, end) = sel.sorted();
            if start.line == end.line {
                self.lines[start.line][start.col..end.col].to_string()
            } else {
                let mut text = String::new();
                text.push_str(&self.lines[start.line][start.col..]);
                text.push('\n');
                for i in (start.line + 1)..end.line {
                    text.push_str(&self.lines[i]);
                    text.push('\n');
                }
                text.push_str(&self.lines[end.line][..end.col]);
                text
            }
        })
    }

    pub fn delete_selection(&mut self) {
        if let Some(sel) = self.selection.clone() {
            let (start, end) = sel.sorted();
            if start.line == end.line {
                self.lines[start.line].drain(start.col..end.col);
            } else {
                self.lines[start.line].truncate(start.col);
                self.lines[end.line].drain(..end.col);
                let right = self.lines[end.line].clone();
                self.lines.drain(start.line + 1..=end.line);
                self.lines[start.line].push_str(&right);
            }
            self.cursor = start;
            self.selection = None;
            self.dirty = true;
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        self.delete_selection();
        let line = self.cursor.line;
        let col = self.cursor.col;
        if ch == '\n' {
            let rest = self.lines[line][col..].to_string();
            self.lines[line].truncate(col);
            self.lines.insert(line + 1, rest);
            self.cursor.line += 1;
            self.cursor.col = 0;
        } else {
            self.lines[line].insert(col, ch);
            self.cursor.col += 1;
        }
        self.dirty = true;
    }

    pub fn insert_str(&mut self, s: &str) {
        for ch in s.chars() {
            self.insert_char(ch);
        }
    }

    pub fn delete_backward(&mut self) {
        if self.selection.is_some() {
            self.delete_selection();
            return;
        }
        if self.cursor.col > 0 {
            let line = self.cursor.line;
            self.lines[line].remove(self.cursor.col - 1);
            self.cursor.col -= 1;
        } else if self.cursor.line > 0 {
            let col = self.lines[self.cursor.line - 1].len();
            let rest = self.lines.remove(self.cursor.line);
            self.lines[self.cursor.line - 1].push_str(&rest);
            self.cursor.line -= 1;
            self.cursor.col = col;
        }
        self.dirty = true;
    }

    pub fn delete_forward(&mut self) {
        if self.selection.is_some() {
            self.delete_selection();
            return;
        }
        let line = self.cursor.line;
        let col = self.cursor.col;
        if col < self.lines[line].len() {
            self.lines[line].remove(col);
            self.dirty = true;
        } else if line + 1 < self.lines.len() {
            let next = self.lines.remove(line + 1);
            self.lines[line].push_str(&next);
            self.dirty = true;
        }
    }

    pub fn move_left(&mut self) {
        self.cursors.clear();
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
        } else if self.cursor.line > 0 {
            self.cursor.line -= 1;
            self.cursor.col = self.lines[self.cursor.line].len();
        }
    }

    pub fn move_right(&mut self) {
        self.cursors.clear();
        let line = self.cursor.line;
        if self.cursor.col < self.lines[line].len() {
            self.cursor.col += 1;
        } else if line + 1 < self.lines.len() {
            self.cursor.line += 1;
            self.cursor.col = 0;
        }
    }

    pub fn move_up(&mut self) {
        self.cursors.clear();
        if self.cursor.line > 0 {
            self.cursor.line -= 1;
            let max = self.lines[self.cursor.line].len();
            if self.cursor.col > max {
                self.cursor.col = max;
            }
        }
    }

    pub fn move_down(&mut self) {
        self.cursors.clear();
        if self.cursor.line + 1 < self.lines.len() {
            self.cursor.line += 1;
            let max = self.lines[self.cursor.line].len();
            if self.cursor.col > max {
                self.cursor.col = max;
            }
        }
    }

    pub fn move_home(&mut self) {
        self.cursors.clear();
        self.cursor.col = 0;
    }

    pub fn move_end(&mut self) {
        self.cursors.clear();
        self.cursor.col = self.lines[self.cursor.line].len();
    }

    pub fn move_page_up(&mut self, height: usize) {
        self.cursors.clear();
        for _ in 0..height.saturating_sub(1) {
            if self.cursor.line == 0 { break; }
            self.cursor.line -= 1;
            let max = self.lines[self.cursor.line].len();
            if self.cursor.col > max {
                self.cursor.col = max;
            }
        }
    }

    pub fn move_page_down(&mut self, height: usize) {
        self.cursors.clear();
        let max_line = self.lines.len().saturating_sub(1);
        for _ in 0..height.saturating_sub(1) {
            if self.cursor.line >= max_line { break; }
            self.cursor.line += 1;
            let max = self.lines[self.cursor.line].len();
            if self.cursor.col > max {
                self.cursor.col = max;
            }
        }
    }

    pub fn add_cursor_alt_up(&mut self) {
        let mut c = self.cursor;
        if c.line > 0 {
            c.line -= 1;
            let max = self.lines[c.line].len();
            if c.col > max { c.col = max; }
            self.cursors.push(c);
        }
    }

    pub fn add_cursor_alt_down(&mut self) {
        let mut c = self.cursor;
        if c.line + 1 < self.lines.len() {
            c.line += 1;
            let max = self.lines[c.line].len();
            if c.col > max { c.col = max; }
            self.cursors.push(c);
        }
    }

    pub fn find_all(&self, query: &str) -> Vec<SearchMatch> {
        if query.is_empty() {
            return Vec::new();
        }
        let mut matches = Vec::new();
        for (i, line) in self.lines.iter().enumerate() {
            let mut start = 0;
            while let Some(pos) = line[start..].to_lowercase().find(&query.to_lowercase()) {
                let abs_pos = start + pos;
                matches.push(SearchMatch {
                    line: i,
                    start_col: abs_pos,
                    end_col: abs_pos + query.len(),
                });
                start = abs_pos + 1;
            }
        }
        matches
    }

    pub fn go_to_match(&mut self, m: &SearchMatch) {
        self.cursor = Cursor::new(m.line, m.start_col);
    }

    pub fn add_extra_cursor(&mut self, c: Cursor) {
        if c != self.cursor && !self.cursors.contains(&c) {
            self.cursors.push(c);
        }
    }

    pub fn clear_extra_cursors(&mut self) {
        self.cursors.clear();
    }
}

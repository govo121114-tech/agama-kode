use std::path::{Path, PathBuf};
use std::fs;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

use crate::theme;

#[derive(Clone, Debug)]
pub struct FileNode {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub depth: usize,
    pub expanded: bool,
}

pub struct FileTree {
    nodes: Vec<FileNode>,
    state: ListState,
    root: PathBuf,
    scroll: usize,
}

impl FileTree {
    pub fn new(root: &Path) -> Self {
        let mut ft = Self {
            nodes: Vec::new(),
            state: ListState::default(),
            root: root.to_path_buf(),
            scroll: 0,
        };
        ft.scan();
        ft.state.select(Some(0));
        ft
    }

    pub fn scan(&mut self) {
        self.nodes.clear();
        self.scan_dir(&self.root.clone(), 0, true);
    }

    fn scan_dir(&mut self, dir: &Path, depth: usize, expanded: bool) {
        if depth > 0 && !expanded {
            return;
        }

        if depth == 0 {
            let name = dir
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| String::from("~"));
            self.nodes.push(FileNode {
                path: dir.to_path_buf(),
                name,
                is_dir: true,
                depth: 0,
                expanded: true,
            });
        }

        if !expanded {
            return;
        }

        let mut entries: Vec<_> = match fs::read_dir(dir) {
            Ok(rd) => rd
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    !name.starts_with('.') || name == ".gitignore"
                })
                .collect(),
            Err(_) => return,
        };

        entries.sort_by_key(|e| {
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            (!is_dir, e.file_name().to_string_lossy().to_string())
        });

        for entry in entries {
            let path = entry.path();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let name = entry.file_name().to_string_lossy().to_string();
            let was_expanded = self.is_expanded(&path);

            self.nodes.push(FileNode {
                path: path.clone(),
                name,
                is_dir,
                depth: depth + 1,
                expanded: was_expanded,
            });

            if is_dir && was_expanded {
                self.scan_dir(&path, depth + 1, was_expanded);
            }
        }
    }

    fn is_expanded(&self, path: &Path) -> bool {
        self.nodes.iter().any(|n| n.path == path && n.is_dir && n.expanded)
    }

    pub fn toggle_expand(&mut self) {
        if let Some(idx) = self.state.selected() {
            if idx < self.nodes.len() && self.nodes[idx].is_dir {
                self.nodes[idx].expanded = !self.nodes[idx].expanded;
                self.scan();
                self.state.select(Some(idx));
            }
        }
    }

    pub fn selected_path(&self) -> Option<PathBuf> {
        self.state.selected().and_then(|idx| {
            self.nodes.get(idx).map(|n| n.path.clone())
        })
    }

    pub fn select_next(&mut self) {
        let len = self.nodes.len();
        if len == 0 { return; }
        let i = self.state.selected().unwrap_or(0);
        self.state.select(Some((i + 1).min(len - 1)));
    }

    pub fn select_prev(&mut self) {
        let len = self.nodes.len();
        if len == 0 { return; }
        let i = self.state.selected().unwrap_or(0);
        self.state.select(Some(i.saturating_sub(1)));
    }

    pub fn render(&mut self, _area: Rect) -> (List, &mut ListState) {
        let items: Vec<ListItem> = self
            .nodes
            .iter()
            .map(|n| {
                let prefix = if n.is_dir {
                    if n.expanded { "▼ " } else { "▶ " }
                } else {
                    "  "
                };
                let indent = "  ".repeat(n.depth);
                let content = if n.is_dir {
                    Span::styled(
                        format!("{}{}{}", indent, prefix, n.name),
                        Style::default().fg(theme::TREE_DIR),
                    )
                } else {
                    Span::styled(
                        format!("{}{}", indent, n.name),
                        Style::default().fg(theme::TREE_FILE),
                    )
                };
                ListItem::new(Line::from(content))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::RIGHT)
                    .style(Style::default().bg(theme::BG)),
            )
            .highlight_style(
                Style::default()
                    .bg(theme::TREE_SELECTED_BG)
                    .fg(theme::FG_BRIGHT),
            );

        (list, &mut self.state)
    }
}

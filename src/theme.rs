use ratatui::style::{Color, Style};

pub const BG: Color = Color::Rgb(30, 30, 30);
pub const BG_DARK: Color = Color::Rgb(22, 22, 22);
pub const BG_LIGHT: Color = Color::Rgb(45, 45, 45);
pub const BG_ACTIVE: Color = Color::Rgb(55, 55, 55);
pub const FG: Color = Color::Rgb(220, 220, 220);
pub const FG_DIM: Color = Color::Rgb(140, 140, 140);
pub const FG_BRIGHT: Color = Color::Rgb(255, 255, 255);
pub const ACCENT: Color = Color::Rgb(86, 156, 214);
pub const ACCENT_GREEN: Color = Color::Rgb(106, 190, 130);
pub const ACCENT_ORANGE: Color = Color::Rgb(206, 145, 90);
pub const ACCENT_RED: Color = Color::Rgb(206, 90, 90);
pub const LINE_NUM: Color = Color::Rgb(100, 100, 100);
pub const LINE_NUM_ACTIVE: Color = Color::Rgb(200, 200, 200);
pub const SELECTION_BG: Color = Color::Rgb(50, 80, 120);
pub const SEARCH_MATCH_BG: Color = Color::Rgb(100, 80, 30);
pub const CURSOR_COLOR: Color = Color::Rgb(200, 200, 200);
pub const TREE_DIR: Color = Color::Rgb(86, 156, 214);
pub const TREE_FILE: Color = Color::Rgb(220, 220, 220);
pub const TREE_SELECTED_BG: Color = Color::Rgb(50, 80, 120);
pub const TAB_ACTIVE_BG: Color = Color::Rgb(45, 45, 45);
pub const TAB_INACTIVE_BG: Color = Color::Rgb(30, 30, 30);
pub const STATUS_BG: Color = Color::Rgb(22, 22, 22);

pub fn line_num_style(active: bool) -> Style {
    if active {
        Style::default().fg(LINE_NUM_ACTIVE).bg(BG_DARK)
    } else {
        Style::default().fg(LINE_NUM).bg(BG_DARK)
    }
}

pub fn cursor_style() -> Style {
    Style::default().bg(CURSOR_COLOR).fg(BG)
}

pub fn selection_style() -> Style {
    Style::default().bg(SELECTION_BG)
}

pub fn search_match_style() -> Style {
    Style::default().bg(SEARCH_MATCH_BG).fg(FG_BRIGHT)
}

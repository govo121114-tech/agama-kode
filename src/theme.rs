use ratatui::style::{Color, Style, Modifier};

// Modern dark theme with improved contrast
pub const BG: Color = Color::Rgb(28, 30, 40);
pub const BG_DARK: Color = Color::Rgb(20, 22, 30);
pub const BG_LIGHT: Color = Color::Rgb(38, 40, 52);
pub const BG_ACTIVE: Color = Color::Rgb(50, 52, 68);
pub const BG_HOVER: Color = Color::Rgb(45, 47, 60);

// Foreground colors
pub const FG: Color = Color::Rgb(200, 205, 220);
pub const FG_DIM: Color = Color::Rgb(120, 125, 140);
pub const FG_BRIGHT: Color = Color::Rgb(235, 240, 255);
pub const FG_MUTED: Color = Color::Rgb(90, 95, 110);

// Accent colors - modern gradient palette
pub const ACCENT: Color = Color::Rgb(116, 166, 255);      // Bright blue
pub const ACCENT_PURPLE: Color = Color::Rgb(180, 120, 255); // Purple
pub const ACCENT_PINK: Color = Color::Rgb(255, 120, 180);   // Pink
pub const ACCENT_GREEN: Color = Color::Rgb(80, 220, 160);   // Fresh green
pub const ACCENT_CYAN: Color = Color::Rgb(80, 220, 220);    // Cyan
pub const ACCENT_ORANGE: Color = Color::Rgb(255, 160, 90);  // Warm orange
pub const ACCENT_RED: Color = Color::Rgb(255, 100, 100);    // Soft red
pub const ACCENT_YELLOW: Color = Color::Rgb(255, 200, 80);  // Golden yellow

// Special UI colors
pub const LINE_NUM: Color = Color::Rgb(80, 85, 100);
pub const LINE_NUM_ACTIVE: Color = Color::Rgb(180, 185, 200);
pub const SELECTION_BG: Color = Color::Rgb(50, 60, 90);
pub const SEARCH_MATCH_BG: Color = Color::Rgb(120, 90, 40);
pub const SEARCH_MATCH_CURRENT: Color = Color::Rgb(180, 120, 50);
pub const CURSOR_COLOR: Color = Color::Rgb(180, 190, 220);
pub const BORDER_COLOR: Color = Color::Rgb(60, 65, 85);
pub const BORDER_FOCUS: Color = Color::Rgb(116, 166, 255);

// File tree colors
pub const TREE_DIR: Color = Color::Rgb(140, 170, 220);
pub const TREE_FILE: Color = Color::Rgb(200, 205, 220);
pub const TREE_SELECTED_BG: Color = Color::Rgb(50, 60, 90);
pub const TREE_ICON_DIR: Color = Color::Rgb(255, 200, 80);
pub const TREE_ICON_FILE: Color = Color::Rgb(100, 180, 220);

// Tab colors
pub const TAB_ACTIVE_BG: Color = Color::Rgb(38, 40, 52);
pub const TAB_INACTIVE_BG: Color = Color::Rgb(28, 30, 40);
pub const TAB_HOVER_BG: Color = Color::Rgb(45, 47, 60);
pub const TAB_SEPARATOR: Color = Color::Rgb(50, 55, 70);

// Status bar colors
pub const STATUS_BG: Color = Color::Rgb(20, 22, 30);
pub const STATUS_FG: Color = Color::Rgb(180, 185, 200);
pub const MODE_NORMAL_BG: Color = Color::Rgb(80, 220, 160);
pub const MODE_INSERT_BG: Color = Color::Rgb(116, 166, 255);
pub const MODE_CMD_BG: Color = Color::Rgb(255, 160, 90);
pub const MODE_SEARCH_BG: Color = Color::Rgb(255, 200, 80);
pub const MODE_TREE_BG: Color = Color::Rgb(180, 120, 255);
pub const MODE_TERM_BG: Color = Color::Rgb(255, 120, 180);

// Syntax highlighting base colors (used as fallback)
pub const SYNTAX_KEYWORD: Color = Color::Rgb(255, 120, 180);
pub const SYNTAX_STRING: Color = Color::Rgb(140, 200, 120);
pub const SYNTAX_COMMENT: Color = Color::Rgb(90, 95, 110);
pub const SYNTAX_FUNCTION: Color = Color::Rgb(116, 166, 255);
pub const SYNTAX_TYPE: Color = Color::Rgb(255, 200, 80);
pub const SYNTAX_NUMBER: Color = Color::Rgb(255, 160, 90);
pub const SYNTAX_VARIABLE: Color = Color::Rgb(200, 205, 220);
pub const SYNTAX_CONSTANT: Color = Color::Rgb(140, 180, 255);

pub fn line_num_style(active: bool) -> Style {
    if active {
        Style::default()
            .fg(LINE_NUM_ACTIVE)
            .bg(BG_DARK)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(LINE_NUM).bg(BG_DARK)
    }
}

pub fn cursor_style() -> Style {
    Style::default()
        .bg(CURSOR_COLOR)
        .fg(BG)
        .add_modifier(Modifier::REVERSED)
}

pub fn selection_style() -> Style {
    Style::default()
        .bg(SELECTION_BG)
        .fg(FG_BRIGHT)
}

pub fn search_match_style() -> Style {
    Style::default()
        .bg(SEARCH_MATCH_BG)
        .fg(FG_BRIGHT)
        .add_modifier(Modifier::BOLD)
}

pub fn search_match_current_style() -> Style {
    Style::default()
        .bg(SEARCH_MATCH_CURRENT)
        .fg(BG_DARK)
        .add_modifier(Modifier::BOLD)
}

pub fn border_style(focused: bool) -> Style {
    if focused {
        Style::default().fg(BORDER_FOCUS)
    } else {
        Style::default().fg(BORDER_COLOR)
    }
}

pub fn mode_style(mode: &str) -> Style {
    let bg = match mode {
        "NORMAL" => MODE_NORMAL_BG,
        "INSERT" => MODE_INSERT_BG,
        "CMD" | "COMMAND" => MODE_CMD_BG,
        "SEARCH" => MODE_SEARCH_BG,
        "TREE" => MODE_TREE_BG,
        "TERM" => MODE_TERM_BG,
        _ => ACCENT,
    };
    Style::default().fg(BG_DARK).bg(bg).add_modifier(Modifier::BOLD)
}

pub fn tab_style(is_active: bool, is_hovered: bool) -> Style {
    if is_active {
        Style::default()
            .fg(FG_BRIGHT)
            .bg(TAB_ACTIVE_BG)
            .add_modifier(Modifier::BOLD)
    } else if is_hovered {
        Style::default()
            .fg(FG)
            .bg(TAB_HOVER_BG)
    } else {
        Style::default()
            .fg(FG_DIM)
            .bg(TAB_INACTIVE_BG)
    }
}

pub fn tree_file_style(is_dir: bool, selected: bool) -> Style {
    if selected {
        Style::default()
            .fg(FG_BRIGHT)
            .bg(TREE_SELECTED_BG)
            .add_modifier(Modifier::BOLD)
    } else if is_dir {
        Style::default().fg(TREE_DIR)
    } else {
        Style::default().fg(TREE_FILE)
    }
}

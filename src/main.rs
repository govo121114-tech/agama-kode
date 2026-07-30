mod ai_project;
mod app;
mod buffer;
mod cmd_palette;
mod editor;
mod filetree;
mod search;
mod status;
mod term_panel;
mod theme;

use std::io;
use std::time::{Duration, Instant};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::{backend::CrosstermBackend, Terminal};
use app::{App, Action};

fn main() -> io::Result<()> {
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    stdout.execute(EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = App::new();
    let mut last_key: Option<(KeyCode, KeyModifiers, Instant)> = None;

    while !app.quit {
        terminal.draw(|f| {
            app.render(f);
        })?;

        if event::poll(Duration::from_millis(100))? {
            let evt = event::read()?;

            let skip = match &evt {
                Event::Key(KeyEvent { code, modifiers, .. }) => {
                    if let Some((last_code, last_mods, last_time)) = &last_key {
                        code == last_code
                            && modifiers == last_mods
                            && last_time.elapsed() < Duration::from_millis(50)
                    } else {
                        false
                    }
                }
                _ => false,
            };

            if !skip {
                last_key = match &evt {
                    Event::Key(KeyEvent { code, modifiers, .. }) => {
                        Some((*code, *modifiers, Instant::now()))
                    }
                    _ => None,
                };
                let _ = app.handle_event(evt);
            }
        }

        match app.action {
            Action::Quit => app.quit = true,
            Action::None => {}
        }
    }

    terminal::disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

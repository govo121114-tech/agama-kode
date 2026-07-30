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
use crossterm::event::{self, Event};
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

    while !app.quit {
        terminal.draw(|f| {
            app.render(f);
        })?;

        if let Event::Key(ke) = event::read()? {
            let _ = app.handle_event(Event::Key(ke));
        } else {
            let _ = app.handle_event(event::read()?);
        }

        match app.action {
            Action::Quit => {
                app.quit = true;
            }
            Action::None => {}
        }
    }

    terminal::disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

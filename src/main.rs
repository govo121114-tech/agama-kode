mod app;
mod buffer;
mod editor;
mod filetree;
mod search;
mod status;
mod theme;

use std::io;
use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::{backend::CrosstermBackend, Terminal};
use app::App;

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
            if ke.code == KeyCode::Char('c')
                && ke.modifiers == crossterm::event::KeyModifiers::CONTROL
            {
                // Send Ctrl+C as a normal key, not as interrupt
                let _ = app.handle_event(Event::Key(ke));
            } else {
                let _ = app.handle_event(Event::Key(ke));
            }
        } else {
            let _ = app.handle_event(event::read()?);
        }
    }

    terminal::disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

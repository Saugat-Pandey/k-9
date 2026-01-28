use std::io::{stdout, Result};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};

fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    // Simple app state
    let mut ticks: u64 = 0;

    // Event loop
    loop {
        terminal.draw(|f| {
            let size = f.size();
            let block = Block::default().title("kv_store TUI").borders(Borders::ALL);
            let text = format!(
                "Press q or Esc to quit.\nTicks: {ticks}\n\nYou can embed views here."
            );
            let para = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
            f.render_widget(para, size);
        })?;

        // Poll for input with a small timeout to produce ticks
        if event::poll(std::time::Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    _ => {}
                },
                _ => {}
            }
        }
        ticks += 1;
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}

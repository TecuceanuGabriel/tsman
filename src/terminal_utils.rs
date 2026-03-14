use std::io;

use crossterm::{
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use ratatui::{DefaultTerminal, Terminal, prelude::CrosstermBackend};

use anyhow::Result;

/// Enters raw mode and alternate screen. Must be paired with [`restore`].
pub fn init() -> Result<DefaultTerminal> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Leaves raw mode and alternate screen.
pub fn restore(mut terminal: DefaultTerminal) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

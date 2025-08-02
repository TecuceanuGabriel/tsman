use std::{
    io::{self, Stdout},
    time::Duration,
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use ratatui::{
    DefaultTerminal, Frame, Terminal,
    prelude::CrosstermBackend,
    widgets::{Block, Borders, List, ListState},
};

use anyhow::{Context, Result};

#[derive(Debug)]
pub struct MenuUi {
    all_items: Vec<String>,
    filtered_items: Vec<String>,
    input: String,
    list_state: ListState,
    exit: bool,
}

impl MenuUi {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }

        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let text = "> ".to_string() + &self.input;
        let input_text = ratatui::widgets::Paragraph::new(text).style(
            ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
        );
        frame.render_widget(input_text, frame.area());
    }

    fn handle_events(&mut self) -> Result<()> {
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Esc => self.exit = true,
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }

    pub fn new(items: Vec<String>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            all_items: items.clone(),
            filtered_items: items,
            input: String::new(),
            list_state,
            exit: false,
        }
    }
}

pub fn init() -> Result<DefaultTerminal> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

pub fn restore(mut terminal: DefaultTerminal) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

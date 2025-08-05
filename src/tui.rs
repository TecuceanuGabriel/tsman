use std::{
    fmt,
    io::{self},
    time::Duration,
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use ratatui::{
    DefaultTerminal, Frame, Terminal,
    prelude::CrosstermBackend,
    widgets::{Block, Borders, List, ListState},
};

use anyhow::Result;

#[derive(Debug, Clone, PartialEq)]
pub enum MenuAction {
    Open,
    Edit,
    Delete,
}

pub struct MenuUi {
    all_items: Vec<String>,
    filtered_items: Vec<String>,
    input: String,

    list_state: ListState,
    matcher: SkimMatcherV2,

    selection: Option<String>,
    action: Option<MenuAction>,

    exit: bool,
}

impl fmt::Debug for MenuUi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MenuUi")
            .field("all_items", &self.all_items)
            .field("filtered_items", &self.filtered_items)
            .field("input", &self.input)
            .field("list_state", &self.list_state)
            .field("selection", &self.selection)
            .field("action", &self.action)
            .field("exit", &self.exit)
            .finish()
    }
}

impl MenuUi {
    pub fn new(items: Vec<String>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            all_items: items.clone(),
            filtered_items: items,
            input: String::new(),
            list_state,
            matcher: fuzzy_matcher::skim::SkimMatcherV2::default(),
            selection: None,
            action: None,
            exit: false,
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }

        Ok(())
    }

    pub fn get_selection(&self) -> Option<String> {
        self.selection.clone()
    }

    pub fn get_action(&self) -> Option<MenuAction> {
        self.action.clone()
    }

    fn draw(&mut self, frame: &mut Frame) {
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Min(3),
                ratatui::layout::Constraint::Length(3),
            ])
            .split(frame.area());

        let items = self.filtered_items.iter().map(|s| s.as_str());
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Results"))
            .highlight_style(
                ratatui::style::Style::default()
                    .bg(ratatui::style::Color::Blue),
            );

        frame.render_stateful_widget(list, chunks[0], &mut self.list_state);

        let input_block =
            Block::default().borders(Borders::ALL).title("Search");
        frame.render_widget(input_block, chunks[1]);

        let text = "> ".to_string() + &self.input;
        let input_text = ratatui::widgets::Paragraph::new(text).style(
            ratatui::style::Style::default().fg(ratatui::style::Color::Green),
        );

        frame.render_widget(
            input_text,
            chunks[1].inner(ratatui::layout::Margin {
                horizontal: 1,
                vertical: 1,
            }),
        );
    }

    fn handle_events(&mut self) -> Result<()> {
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                self.handle_key_event(key);
            }
        }

        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('p') => self.move_selection(-1),
                KeyCode::Char('n') => self.move_selection(1),
                KeyCode::Char('e') => self.set_pending_action(MenuAction::Edit),
                KeyCode::Char('d') => {
                    self.set_pending_action(MenuAction::Delete);
                    self.move_selection(-1);
                    self.filtered_items = self.all_items.clone();
                }
                KeyCode::Char('c') => self.exit = true,
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Char(c) => {
                    self.input.push(c);
                    self.update_filter();
                }
                KeyCode::Backspace => {
                    self.input.pop();
                    self.update_filter();
                }
                KeyCode::Up => self.move_selection(-1),
                KeyCode::Down => self.move_selection(1),
                KeyCode::Enter => self.set_pending_action(MenuAction::Open),
                KeyCode::Esc => self.exit = true,
                _ => {}
            }
        }
    }

    fn set_pending_action(&mut self, action: MenuAction) {
        if let Some(selected) = self.list_state.selected() {
            if let Some(item) = self.filtered_items.get(selected) {
                if action == MenuAction::Delete {
                    self.all_items.remove(selected);
                } else {
                    self.exit = true;
                }

                self.selection = Some(item.to_string());
                self.action = Some(action);
            }
        }
    }

    fn update_filter(&mut self) {
        if self.input.is_empty() {
            self.filtered_items = self.all_items.clone();
        } else {
            self.filtered_items = self
                .all_items
                .iter()
                .filter(|item| {
                    self.matcher.fuzzy_match(item, &self.input).is_some()
                })
                .cloned()
                .collect();
        }

        if self.filtered_items.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }

    fn move_selection(&mut self, delta: i32) {
        if let Some(selected) = self.list_state.selected() {
            let new_selected =
                usize::try_from((selected as i32 + delta).max(0)).unwrap_or(0);
            self.list_state.select(Some(
                new_selected.min(self.filtered_items.len().saturating_sub(1)),
            ));
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

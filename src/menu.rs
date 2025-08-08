use std::{
    collections::VecDeque,
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
    layout::{Alignment, Constraint, Direction, Flex, Layout, Rect},
    prelude::CrosstermBackend,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListState, Paragraph},
};

use anyhow::Result;

use crate::persistence::load_session_from_config;
use crate::tmux::session::Session;

#[derive(Debug, Clone, PartialEq)]
pub enum MenuAction {
    Open,
    Edit,
    Delete,
}

#[derive(Debug)]
pub struct MenuActionItem {
    pub selection: String,
    pub action: MenuAction,
}

pub struct MenuUi {
    all_items: Vec<String>,
    filtered_items: Vec<String>,
    input: String,

    list_state: ListState,
    matcher: SkimMatcherV2,

    action_queue: VecDeque<MenuActionItem>,

    ask_for_confirmation: bool,
    show_confirmation_popup: bool,
    show_preview: bool,

    exit: bool,
}

impl fmt::Debug for MenuUi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MenuUi")
            .field("all_items", &self.all_items)
            .field("filtered_items", &self.filtered_items)
            .field("input", &self.input)
            .field("list_state", &self.list_state)
            .field("action_queue", &self.action_queue)
            .field("show_confirmation_popup", &self.show_confirmation_popup)
            .field("show_preview", &self.show_preview)
            .field("exit", &self.exit)
            .finish()
    }
}

impl MenuUi {
    pub fn new(
        items: Vec<String>,
        show_preview: bool,
        ask_for_confirmation: bool,
    ) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            all_items: items.clone(),
            filtered_items: items,
            input: String::new(),
            list_state,
            matcher: fuzzy_matcher::skim::SkimMatcherV2::default(),
            action_queue: VecDeque::new(),
            ask_for_confirmation,
            show_confirmation_popup: false,
            show_preview,
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

    pub fn dequeue_action(&mut self) -> Result<Option<MenuActionItem>> {
        Ok(self.action_queue.pop_front())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let chunks = if self.show_preview {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(60),
                    Constraint::Percentage(40),
                ])
                .split(frame.area())
        } else {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(100)])
                .split(frame.area())
        };

        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(3)])
            .split(chunks[0]);

        let items = self.filtered_items.iter().map(|s| s.as_str());
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Results"))
            .highlight_style(
                ratatui::style::Style::default()
                    .bg(ratatui::style::Color::Blue),
            );

        frame.render_stateful_widget(
            list,
            left_chunks[0],
            &mut self.list_state,
        );

        let input_block =
            Block::default().borders(Borders::ALL).title("Search");
        frame.render_widget(input_block, left_chunks[1]);

        let text = "> ".to_string() + &self.input;
        let input_text = ratatui::widgets::Paragraph::new(text).style(
            ratatui::style::Style::default().fg(ratatui::style::Color::Green),
        );

        frame.render_widget(
            input_text,
            left_chunks[1].inner(ratatui::layout::Margin {
                horizontal: 1,
                vertical: 1,
            }),
        );

        if self.show_preview {
            // TODO: extract to function
            let preview_block =
                Block::default().borders(Borders::ALL).title("Preview");

            let preview_content = self.generate_preview_content();
            let preview = Paragraph::new(preview_content).block(preview_block);

            frame.render_widget(preview, chunks[1]);
        }

        if self.ask_for_confirmation && self.show_confirmation_popup {
            MenuUi::draw_confirmation_popup(frame);
        }
    }

    fn generate_preview_content(&self) -> String {
        if let Some(selection_idx) = self.list_state.selected() {
            if let Some(selection) = self.filtered_items.get(selection_idx) {
                if let Ok(session_str) = load_session_from_config(selection) {
                    let session: Session =
                        serde_yaml::from_str(&session_str).ok().unwrap();
                    return session.get_preview();
                }
            }
        }

        "".to_string()
    }

    fn draw_confirmation_popup(f: &mut Frame) {
        let popup_area = MenuUi::create_centered_rect(f.area(), 15, 3);

        f.render_widget(Clear, popup_area);

        let block = Block::default()
            .title("Confirm")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::DarkGray));

        let paragraph = Paragraph::new(Line::from("y/n"))
            .block(block)
            .alignment(Alignment::Center);

        f.render_widget(paragraph, popup_area);
    }

    fn create_centered_rect(area: Rect, length_x: u16, length_y: u16) -> Rect {
        let vertical =
            Layout::vertical([Constraint::Length(length_y)]).flex(Flex::Center);
        let horizontal = Layout::horizontal([Constraint::Length(length_x)])
            .flex(Flex::Center);
        let [area] = vertical.areas(area);
        let [area] = horizontal.areas(area);
        area
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

        if self.show_confirmation_popup {
            match key.code {
                KeyCode::Char('y' | 'Y') => {
                    self.handle_delete();
                    self.show_confirmation_popup = false;
                }
                KeyCode::Char('n' | 'N') => {
                    self.show_confirmation_popup = false;
                }
                _ => {}
            }
            return;
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('p') => self.move_selection(-1),
                KeyCode::Char('n') => self.move_selection(1),
                KeyCode::Char('e') => self.enqueue_action(MenuAction::Edit),
                KeyCode::Char('d') => {
                    if self.ask_for_confirmation {
                        self.show_confirmation_popup = true;
                    } else {
                        self.handle_delete();
                    }
                }
                KeyCode::Char('c') => self.exit = true,
                KeyCode::Char('t') => self.show_preview = !self.show_preview,
                KeyCode::Char('w') => {
                    self.remove_last_word_from_input();
                    self.update_filter_and_reset();
                }
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Char(c) => {
                    self.input.push(c);
                    self.update_filter_and_reset();
                }
                KeyCode::Backspace => {
                    self.input.pop();
                    self.update_filter_and_reset();
                }
                KeyCode::Up => self.move_selection(-1),
                KeyCode::Down => self.move_selection(1),
                KeyCode::Enter => self.enqueue_action(MenuAction::Open),
                KeyCode::Esc => self.exit = true,
                _ => {}
            }
        }
    }

    fn handle_delete(&mut self) {
        if let Some(selection_idx) = self.list_state.selected() {
            let selection = match self.filtered_items.get(selection_idx) {
                Some(s) => s.clone(),
                None => return,
            };

            self.enqueue_action(MenuAction::Delete);

            self.all_items.retain(|s| s != &selection);
            self.update_filter();
            self.list_state
                .select(Some(selection_idx.saturating_sub(1)));
        }
    }

    fn enqueue_action(&mut self, action: MenuAction) {
        if let Some(selection_idx) = self.list_state.selected() {
            if let Some(selection) = self.filtered_items.get(selection_idx) {
                if action != MenuAction::Delete {
                    self.exit = true;
                }

                self.action_queue.push_back(MenuActionItem {
                    selection: selection.to_string(),
                    action,
                });
            }
        }
    }

    fn update_filter_and_reset(&mut self) {
        self.update_filter();
        self.reset_position();
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
    }

    fn reset_position(&mut self) {
        if self.filtered_items.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }

    fn move_selection(&mut self, delta: i32) {
        if let Some(selection_idx) = self.list_state.selected() {
            let new_selected =
                usize::try_from((selection_idx as i32 + delta).max(0))
                    .unwrap_or(0);
            self.list_state.select(Some(
                new_selected.min(self.filtered_items.len().saturating_sub(1)),
            ));
        }
    }

    fn remove_last_word_from_input(&mut self) {
        if let Some(last_space) = self.input.trim_end().rfind(' ') {
            self.input.truncate(last_space);
        } else {
            self.input.clear();
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

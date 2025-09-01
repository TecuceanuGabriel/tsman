//! TUI menu
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
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Direction, Flex, Layout, Margin, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListState, Paragraph},
};

use anyhow::Result;

pub mod item;

use crate::menu::item::MenuItem;
use crate::tmux::{self, session::Session};
use crate::{actions, persistence::load_session_from_config};

/// Menu state.
pub struct MenuUi {
    /// List of all items in the menu.
    all_items: Vec<MenuItem>,
    /// List of filtered items using fuzzy-matcher based on the input.
    filtered_items: Vec<MenuItem>,
    /// Input used for filtering.
    input: String,

    ask_for_confirmation: bool,
    show_confirmation_popup: bool,
    show_preview: bool,
    show_help: bool,
    exit: bool,

    list_state: ListState,
    matcher: SkimMatcherV2,
}

impl fmt::Debug for MenuUi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MenuUi")
            .field("all_items", &self.all_items)
            .field("filtered_items", &self.filtered_items)
            .field("input", &self.input)
            .field("list_state", &self.list_state)
            .field("show_confirmation_popup", &self.show_confirmation_popup)
            .field("show_preview", &self.show_preview)
            .field("show_help", &self.show_help)
            .field("exit", &self.exit)
            .finish()
    }
}

impl MenuUi {
    /// Creates a new [`MenuUi`] instance.
    ///
    /// # Arguments
    /// * `items` - The list of menu items to display.
    /// * `show_preview` - Whether to show the preview pane.
    /// * `ask_for_confirmation` - Whether to require confirmation before
    ///   deleting.
    pub fn new(
        items: Vec<MenuItem>,
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
            ask_for_confirmation,
            show_confirmation_popup: false,
            show_preview,
            show_help: false,
            exit: false,
        }
    }

    /// Runs the menu loop until the user exits.
    ///
    /// # Arguments
    /// * `terminal` - The terminal backend to draw on.
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events(terminal)?;
        }

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Length(1), // help hint
            ])
            .split(frame.area());

        let content_chunks = if self.show_preview {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(60),
                    Constraint::Percentage(40),
                ])
                .split(main_chunks[0])
        } else {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(100)])
                .split(main_chunks[0])
        };

        let left_content = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(3)])
            .split(content_chunks[0]);

        let results_block =
            Block::default().borders(Borders::ALL).title("Results");
        let items: Vec<String> =
            self.filtered_items.iter().map(|s| s.to_string()).collect();

        if items.is_empty() {
            frame.render_widget(
                Paragraph::new("No results...")
                    .block(results_block)
                    .style(Style::default().fg(Color::DarkGray)),
                left_content[0],
            );
        } else {
            let list = List::new(items)
                .block(results_block)
                .highlight_style(Style::default().bg(Color::Blue));

            frame.render_stateful_widget(
                list,
                left_content[0],
                &mut self.list_state,
            );
        }

        let input_block =
            Block::default().borders(Borders::ALL).title("Search");

        frame.render_widget(input_block, left_content[1]);

        let input_text = Paragraph::new("> ".to_string() + &self.input)
            .style(Style::default().fg(Color::Green));

        frame.render_widget(
            input_text,
            left_content[1].inner(Margin {
                horizontal: 1,
                vertical: 1,
            }),
        );

        let help_hint = Paragraph::new("C-h: Help | Esc: Quit")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));

        frame.render_widget(help_hint, main_chunks[1]);

        if self.show_preview {
            self.draw_preview_pane(frame, content_chunks[1]);
        }

        if self.ask_for_confirmation && self.show_confirmation_popup {
            MenuUi::draw_confirmation_popup(frame);
        }

        if self.show_help {
            MenuUi::draw_help_popup(frame);
        }
    }

    fn generate_preview_content(&self) -> String {
        if let Some(selection_idx) = self.list_state.selected()
            && let Some(selection) = self.filtered_items.get(selection_idx)
            && let Ok(session_str) = load_session_from_config(&selection.name)
        {
            let session: Session =
                serde_yaml::from_str(&session_str).ok().unwrap();
            return session.get_preview();
        }

        "".to_string()
    }

    fn draw_preview_pane(&self, frame: &mut Frame, chunk: Rect) {
        let preview_block =
            Block::default().borders(Borders::ALL).title("Preview");

        let preview_content = self.generate_preview_content();
        let preview = Paragraph::new(preview_content).block(preview_block);

        frame.render_widget(preview, chunk);
    }

    fn draw_confirmation_popup(f: &mut Frame) {
        let popup_area = MenuUi::create_centered_rect(f.area(), 15, 3);

        f.render_widget(Clear, popup_area);

        let block = Block::default()
            .title("Confirm")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::DarkGray));

        let paragraph = Paragraph::new(Line::from("Y/n"))
            .block(block)
            .alignment(Alignment::Center);

        f.render_widget(paragraph, popup_area);
    }

    fn draw_help_popup(f: &mut Frame) {
        let popup_area = MenuUi::create_centered_rect(f.area(), 60, 14);

        f.render_widget(Clear, popup_area);

        let navigation_block = Block::default()
            .title("Navigation")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::DarkGray));

        let session_block = Block::default()
            .title("Session Actions")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::DarkGray));

        let ui_block = Block::default()
            .title("UI Controls")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::DarkGray));

        let popup_block = Block::default()
            .title("Popup")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::DarkGray));

        let navigation_text = vec![
            Line::from("Esc/C-c → Close"),
            Line::from("↑/C-p   → Previous item"),
            Line::from("↓/C-n   → Next item"),
        ];

        let session_text = vec![
            Line::from("C-e   → Edit session"),
            Line::from("C-d   → Delete/kill"),
            Line::from("C-s   → Save session"),
            Line::from("C-k   → Kill session"),
            Line::from("Enter → Open session"),
        ];

        let ui_text = vec![
            Line::from("C-t → Toggle preview"),
            Line::from("C-h → Toggle help"),
            Line::from("C-w → Delete last word"),
        ];

        let popup_text = vec![
            Line::from("y/Y/Enter → Confirm"),
            Line::from("n/N/Esc/q → Abort"),
        ];

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(7), Constraint::Length(7)])
            .split(popup_area);

        let top_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(chunks[0]);

        let bottom_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(chunks[1]);

        f.render_widget(
            Paragraph::new(navigation_text).block(navigation_block),
            top_chunks[0],
        );
        f.render_widget(
            Paragraph::new(session_text).block(session_block),
            top_chunks[1],
        );
        f.render_widget(
            Paragraph::new(ui_text).block(ui_block),
            bottom_chunks[0],
        );
        f.render_widget(
            Paragraph::new(popup_text).block(popup_block),
            bottom_chunks[1],
        );
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

    fn handle_events(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
        {
            self.handle_key_event(key, terminal)?;
        }

        Ok(())
    }

    fn handle_key_event(
        &mut self,
        key: KeyEvent,
        terminal: &mut DefaultTerminal,
    ) -> Result<()> {
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }

        if self.show_confirmation_popup {
            match key.code {
                KeyCode::Char('y' | 'Y') | KeyCode::Enter => {
                    self.handle_delete()?;
                    self.show_confirmation_popup = false;
                }
                KeyCode::Char('n' | 'N' | 'q') | KeyCode::Esc => {
                    self.show_confirmation_popup = false;
                }
                _ => {}
            }
            return Ok(());
        }

        if self.show_help {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                if let KeyCode::Char('h' | 'c') = key.code {
                    self.show_help = !self.show_help
                }
            } else {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => {
                        self.show_help = !self.show_help
                    }
                    _ => {}
                }
            }
            return Ok(());
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('p') => self.move_selection(-1),
                KeyCode::Char('n') => self.move_selection(1),
                KeyCode::Char('e') => self.handle_edit(terminal)?,
                KeyCode::Char('s') => self.handle_save()?,
                KeyCode::Char('d') => {
                    if self.ask_for_confirmation {
                        self.show_confirmation_popup = true;
                    } else {
                        self.handle_delete()?;
                    }
                }
                KeyCode::Char('k') => self.handle_kill()?,
                KeyCode::Char('c') => self.exit = true,
                KeyCode::Char('t') => self.show_preview = !self.show_preview,
                KeyCode::Char('h') => self.show_help = !self.show_help,
                KeyCode::Char('w') => {
                    self.remove_last_word_from_input();
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
                KeyCode::Enter => self.handle_open()?,
                KeyCode::Esc => self.exit = true,
                _ => {}
            }
        }

        Ok(())
    }

    fn handle_open(&mut self) -> Result<()> {
        if let Some(selection_idx) = self.list_state.selected() {
            let selection = match self.filtered_items.get(selection_idx) {
                Some(s) => s.clone(),
                None => return Ok(()),
            };

            actions::open(&selection.name)?;
            self.exit = true;
        }

        Ok(())
    }

    fn handle_delete(&mut self) -> Result<()> {
        if let Some(selection_idx) = self.list_state.selected() {
            let selection = match self.filtered_items.get(selection_idx) {
                Some(s) => s.clone(),
                None => return Ok(()),
            };

            if selection.saved {
                actions::delete(&selection.name)?;
                self.update_menu_item(&selection.name, Some(false), None);
            } else {
                tmux::interface::close_session(&selection.name)?;
                self.update_menu_item(&selection.name, None, Some(false));
            }

            if (selection.saved && !selection.active)
                || (!selection.saved && selection.active)
            {
                self.all_items.retain(|item| item.name != selection.name);
                self.list_state
                    .select(Some(selection_idx.saturating_sub(1)));
            }

            self.update_filter();
        }

        Ok(())
    }

    fn handle_edit(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        if let Some(selection_idx) = self.list_state.selected() {
            let selection = match self.filtered_items.get(selection_idx) {
                Some(s) => s.clone(),
                None => return Ok(()),
            };

            if selection.saved {
                disable_raw_mode()?;
                execute!(io::stdout(), LeaveAlternateScreen)?;
                actions::edit(Some(&selection.name))?;
                enable_raw_mode()?;
                execute!(io::stdout(), EnterAlternateScreen)?;
                terminal.clear()?;
            }
        }

        Ok(())
    }

    fn handle_save(&mut self) -> Result<()> {
        if let Some(selection_idx) = self.list_state.selected() {
            let selection = match self.filtered_items.get(selection_idx) {
                Some(s) => s.clone(),
                None => return Ok(()),
            };

            if !selection.saved {
                actions::save_target(&selection.name)?;
                self.update_menu_item(&selection.name, Some(true), None);
                self.update_filter();
            }
        }

        Ok(())
    }

    fn handle_kill(&mut self) -> Result<()> {
        if let Some(selection_idx) = self.list_state.selected() {
            let selection = match self.filtered_items.get(selection_idx) {
                Some(s) => s.clone(),
                None => return Ok(()),
            };

            if selection.active {
                tmux::interface::close_session(&selection.name)?;
                self.update_menu_item(&selection.name, None, Some(false));

                if !selection.saved {
                    self.all_items.retain(|item| item.name != selection.name);
                    self.list_state
                        .select(Some(selection_idx.saturating_sub(1)));
                }

                self.update_filter();
            }
        }

        return Ok(());
    }

    fn update_menu_item(
        &mut self,
        name: &str,
        saved: Option<bool>,
        active: Option<bool>,
    ) {
        if let Some(item) = self.all_items.iter_mut().find(|i| i.name == name) {
            if let Some(saved_val) = saved {
                item.saved = saved_val;
            }
            if let Some(active_val) = active {
                item.active = active_val;
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
                    self.matcher.fuzzy_match(&item.name, &self.input).is_some()
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
        if self.input.is_empty() {
            return;
        }

        if let Some(last_space) = self.input.trim_end().rfind(' ') {
            self.input.truncate(last_space);
        } else {
            self.input.clear();
        }

        self.update_filter_and_reset();
    }
}

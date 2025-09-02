//! TUI menu
use std::{
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

use ratatui::DefaultTerminal;

use anyhow::Result;

pub mod item;
pub mod items_state;
pub mod menu_state;
pub mod renderer;
pub mod ui_flags;

use crate::menu::item::MenuItem;
use crate::menu::menu_state::MenuState;
use crate::menu::renderer::*;
use crate::{actions, tmux};

/// Menu state.

pub struct Menu {
    state: MenuState,
    renderer: Box<dyn MenuRenderer>,
}

impl Menu {
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
        renderer: Box<dyn MenuRenderer>,
    ) -> Self {
        Self {
            state: MenuState::new(items, show_preview, ask_for_confirmation),
            renderer,
        }
    }

    /// Runs the menu loop until the user exits.
    ///
    /// # Arguments
    /// * `terminal` - The terminal backend to draw on.
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.state.should_exit {
            terminal
                .draw(|frame| self.renderer.draw(frame, &mut self.state))?;
            self.handle_events(terminal)?;
        }

        Ok(())
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

        if self.state.ui_flags.show_confirmation_popup {
            return self.handle_confirmation_popup_key(key);
        }

        if self.state.ui_flags.show_help {
            return self.handle_help_popup_key(key);
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) {
            self.handle_modifier_key_combo(key, terminal)
        } else {
            self.handle_regular_key(key)
        }
    }

    fn handle_confirmation_popup_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('y' | 'Y') | KeyCode::Enter => {
                self.handle_delete()?;
                self.state.ui_flags.show_confirmation_popup = false;
            }
            KeyCode::Char('n' | 'N' | 'q') | KeyCode::Esc => {
                self.state.ui_flags.show_confirmation_popup = false;
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_help_popup_key(&mut self, key: KeyEvent) -> Result<()> {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            if let KeyCode::Char('h' | 'c') = key.code {
                self.state.ui_flags.show_help = !self.state.ui_flags.show_help
            }
        } else {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => {
                    self.state.ui_flags.show_help =
                        !self.state.ui_flags.show_help
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn handle_modifier_key_combo(
        &mut self,
        key: KeyEvent,
        terminal: &mut DefaultTerminal,
    ) -> Result<()> {
        match key.code {
            KeyCode::Char('p') => self.state.items.move_selection(-1),
            KeyCode::Char('n') => self.state.items.move_selection(1),
            KeyCode::Char('e') => self.handle_edit(terminal)?,
            KeyCode::Char('s') => self.handle_save()?,
            KeyCode::Char('d') => {
                if self.state.ui_flags.ask_for_confirmation {
                    self.state.ui_flags.show_confirmation_popup = true;
                } else {
                    self.handle_delete()?;
                }
            }
            KeyCode::Char('k') => self.handle_kill()?,
            KeyCode::Char('c') => self.state.should_exit = true,
            KeyCode::Char('t') => {
                self.state.ui_flags.show_preview =
                    !self.state.ui_flags.show_preview
            }
            KeyCode::Char('h') => {
                self.state.ui_flags.show_help = !self.state.ui_flags.show_help
            }
            KeyCode::Char('w') => {
                self.state.items.remove_last_word_from_input();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_regular_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char(c) => {
                self.state.items.input.push(c);
                self.state.items.update_filter_and_reset();
            }
            KeyCode::Backspace => {
                self.state.items.input.pop();
                self.state.items.update_filter_and_reset();
            }
            KeyCode::Up => self.state.items.move_selection(-1),
            KeyCode::Down => self.state.items.move_selection(1),
            KeyCode::Enter => self.handle_open()?,
            KeyCode::Esc => self.state.should_exit = true,
            _ => {}
        }

        Ok(())
    }

    fn handle_open(&mut self) -> Result<()> {
        if let Some(selection_idx) = self.state.items.list_state.selected() {
            let selection =
                match self.state.items.filtered_items.get(selection_idx) {
                    Some(s) => s.clone(),
                    None => return Ok(()),
                };

            actions::open(&selection.name)?;
            self.state.should_exit = true;
        }

        Ok(())
    }

    fn handle_delete(&mut self) -> Result<()> {
        if let Some(selection_idx) = self.state.items.list_state.selected() {
            let selection =
                match self.state.items.filtered_items.get(selection_idx) {
                    Some(s) => s.clone(),
                    None => return Ok(()),
                };

            if selection.saved {
                actions::delete(&selection.name)?;
                self.state.items.update_item(
                    &selection.name,
                    Some(false),
                    None,
                );
            } else {
                tmux::interface::close_session(&selection.name)?;
                self.state.items.update_item(
                    &selection.name,
                    None,
                    Some(false),
                );
            }

            if (selection.saved && !selection.active)
                || (!selection.saved && selection.active)
            {
                self.state
                    .items
                    .all_items
                    .retain(|item| item.name != selection.name);
                self.state
                    .items
                    .list_state
                    .select(Some(selection_idx.saturating_sub(1)));
            }

            self.state.items.update_filter();
        }

        Ok(())
    }

    fn handle_edit(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        if let Some(selection_idx) = self.state.items.list_state.selected() {
            let selection =
                match self.state.items.filtered_items.get(selection_idx) {
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
        if let Some(selection_idx) = self.state.items.list_state.selected() {
            let selection =
                match self.state.items.filtered_items.get(selection_idx) {
                    Some(s) => s.clone(),
                    None => return Ok(()),
                };

            if !selection.saved {
                actions::save_target(&selection.name)?;
                self.state
                    .items
                    .update_item(&selection.name, Some(true), None);
                self.state.items.update_filter();
            }
        }

        Ok(())
    }

    fn handle_kill(&mut self) -> Result<()> {
        if let Some(selection_idx) = self.state.items.list_state.selected() {
            let selection =
                match self.state.items.filtered_items.get(selection_idx) {
                    Some(s) => s.clone(),
                    None => return Ok(()),
                };

            if selection.active {
                tmux::interface::close_session(&selection.name)?;
                self.state.items.update_item(
                    &selection.name,
                    None,
                    Some(false),
                );

                if !selection.saved {
                    self.state
                        .items
                        .all_items
                        .retain(|item| item.name != selection.name);
                    self.state
                        .items
                        .list_state
                        .select(Some(selection_idx.saturating_sub(1)));
                }

                self.state.items.update_filter();
            }
        }

        Ok(())
    }
}

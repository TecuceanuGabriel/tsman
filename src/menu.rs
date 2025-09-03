//! TUI menu
use std::{
    io::{self},
    time::Duration,
};

use crossterm::{
    event::{self},
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};

use ratatui::DefaultTerminal;

use anyhow::Result;

pub mod event_handler;
pub mod item;
pub mod items_state;
pub mod menu_state;
pub mod renderer;
pub mod ui_flags;

use crate::menu::event_handler::*;
use crate::menu::item::MenuItem;
use crate::menu::menu_state::MenuState;
use crate::menu::renderer::*;
use crate::{actions, tmux};

/// Menu state.

pub struct Menu {
    state: MenuState,
    renderer: Box<dyn MenuRenderer>,
    event_handler: Box<dyn EventHandler>,
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
        event_handler: Box<dyn EventHandler>,
    ) -> Self {
        Self {
            state: MenuState::new(items, show_preview, ask_for_confirmation),
            renderer,
            event_handler,
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
            self.handle_events()?;
        }

        Ok(())
    }

    fn handle_events(&mut self) -> Result<()> {
        if event::poll(Duration::from_millis(50))? {
            let event = event::read()?;
            self.event_handler.handle_event(event, &self.state);
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

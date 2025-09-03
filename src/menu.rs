//! TUI menu
use std::time::Duration;

use crossterm::event::{self};

use ratatui::DefaultTerminal;

use anyhow::Result;

pub mod action_dispacher;
pub mod event_handler;
pub mod item;
pub mod items_state;
pub mod menu_action;
pub mod menu_state;
pub mod renderer;
pub mod ui_flags;

use crate::menu::action_dispacher::*;
use crate::menu::event_handler::*;
use crate::menu::item::MenuItem;
use crate::menu::menu_state::MenuState;
use crate::menu::renderer::*;

/// Menu state.

pub struct Menu {
    state: MenuState,
    renderer: Box<dyn MenuRenderer>,
    event_handler: Box<dyn EventHandler>,
    action_dispacher: Box<dyn ActionDispacher>,
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
        action_dispacher: Box<dyn ActionDispacher>,
    ) -> Self {
        Self {
            state: MenuState::new(items, show_preview, ask_for_confirmation),
            renderer,
            event_handler,
            action_dispacher,
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
            let action = self.event_handler.handle_event(event, &self.state);
            self.action_dispacher.dispach(action, &mut self.state)?;
        }

        Ok(())
    }
}

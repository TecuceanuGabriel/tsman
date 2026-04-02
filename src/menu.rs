//! Interactive TUI menu for managing sessions and layouts.
use std::time::Duration;

use crossterm::event::{self};

use ratatui::DefaultTerminal;

use anyhow::Result;

pub mod action;
pub mod action_dispatcher;
pub mod event_handler;
pub mod item;
pub mod items_state;
pub mod renderer;
pub mod state;
pub mod ui_flags;

use crate::menu::action_dispatcher::*;
use crate::menu::event_handler::*;
use crate::menu::item::MenuItem;
use crate::menu::renderer::*;
use crate::menu::state::MenuState;

/// Top-level menu that owns state, renderer, event handler, and action dispatcher.
pub struct Menu<'a> {
    state: MenuState<'a>,
    renderer: Box<dyn MenuRenderer>,
    event_handler: Box<dyn EventHandler>,
    action_dispacher: Box<dyn ActionDispatcher>,
}

impl<'a> Menu<'a> {
    /// Creates a new [`Menu`] with the given items and configuration.
    pub fn new(
        items: Vec<MenuItem>,
        show_preview: bool,
        ask_for_confirmation: bool,
        current_session: Option<&str>,
        renderer: Box<dyn MenuRenderer>,
        event_handler: Box<dyn EventHandler>,
        action_dispacher: Box<dyn ActionDispatcher>,
    ) -> Self {
        Self {
            state: MenuState::new(
                items,
                show_preview,
                ask_for_confirmation,
                current_session,
            ),
            renderer,
            event_handler,
            action_dispacher,
        }
    }

    /// Runs the render/event loop until the user exits.
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.state.should_exit {
            terminal
                .draw(|frame| self.renderer.draw(frame, &mut self.state))?;

            if event::poll(Duration::from_millis(50))? {
                let event = event::read()?;
                let (action, key_label) =
                    self.event_handler.handle_event(event, &self.state);
                if let Some(label) = key_label {
                    self.state.set_last_key(label);
                }
                self.action_dispacher.dispach(
                    action,
                    &mut self.state,
                    terminal,
                )?;
            }
        }

        Ok(())
    }
}

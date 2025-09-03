use std::io;

use crate::menu::menu_action::MenuAction;
use crate::{actions, menu::menu_state::MenuState, tmux};

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use ratatui::DefaultTerminal;

pub trait ActionDispacher {
    fn dispach(&self, action: MenuAction, state: &mut MenuState);
}

pub struct DefaultActionDispacher;

impl ActionDispacher for DefaultActionDispacher {
    fn dispach(&self, action: MenuAction, state: &mut MenuState) {}
}

fn handle_open(&mut self) -> Result<()> {
    if let Some(selection_idx) = self.state.items.list_state.selected() {
        let selection = match self.state.items.filtered_items.get(selection_idx)
        {
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
        let selection = match self.state.items.filtered_items.get(selection_idx)
        {
            Some(s) => s.clone(),
            None => return Ok(()),
        };

        if selection.saved {
            actions::delete(&selection.name)?;
            self.state
                .items
                .update_item(&selection.name, Some(false), None);
        } else {
            tmux::interface::close_session(&selection.name)?;
            self.state
                .items
                .update_item(&selection.name, None, Some(false));
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
        let selection = match self.state.items.filtered_items.get(selection_idx)
        {
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
        let selection = match self.state.items.filtered_items.get(selection_idx)
        {
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
        let selection = match self.state.items.filtered_items.get(selection_idx)
        {
            Some(s) => s.clone(),
            None => return Ok(()),
        };

        if selection.active {
            tmux::interface::close_session(&selection.name)?;
            self.state
                .items
                .update_item(&selection.name, None, Some(false));

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

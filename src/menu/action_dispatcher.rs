use std::io;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use ratatui::DefaultTerminal;

use crate::menu::action::MenuAction;
use crate::{actions, menu::state::MenuState, tmux};

pub trait ActionDispatcher {
    fn dispach(
        &self,
        action: MenuAction,
        state: &mut MenuState,
        terminal: &mut DefaultTerminal,
    ) -> Result<()>;
}

pub struct DefaultActionDispacher;

impl ActionDispatcher for DefaultActionDispacher {
    fn dispach(
        &self,
        action: MenuAction,
        state: &mut MenuState,
        terminal: &mut DefaultTerminal,
    ) -> Result<()> {
        match action {
            MenuAction::Open => handle_open(state),
            MenuAction::Delete => handle_delete(state),
            MenuAction::Edit => handle_edit(state, terminal),
            MenuAction::Save => handle_save(state),
            MenuAction::Kill => handle_kill(state),
            MenuAction::MoveSelection(delta) => {
                state.items.move_selection(delta);
                Ok(())
            }
            MenuAction::RemoveLastWord => {
                state.items.remove_last_word_from_input();
                Ok(())
            }
            MenuAction::AppendToInput(c) => {
                state.items.input.push(c);
                state.items.update_filter_and_reset();
                Ok(())
            }
            MenuAction::DeleteFromInput => {
                state.items.input.pop();
                state.items.update_filter_and_reset();
                Ok(())
            }
            MenuAction::TogglePreview => {
                state.ui_flags.show_preview = !state.ui_flags.show_preview;
                Ok(())
            }
            MenuAction::ToggleHelp => {
                state.ui_flags.show_help = !state.ui_flags.show_help;
                Ok(())
            }
            MenuAction::HideConfirmation => {
                state.ui_flags.show_confirmation_popup = false;
                Ok(())
            }
            MenuAction::Exit => {
                state.should_exit = true;
                Ok(())
            }
            MenuAction::NOP => Ok(()),
        }
    }
}

fn handle_open(state: &mut MenuState) -> Result<()> {
    let Some((_, selection)) = state.items.get_selected_item() else {
        return Ok(());
    };

    actions::open(&selection.name)?;
    state.should_exit = true;

    Ok(())
}

fn handle_delete(state: &mut MenuState) -> Result<()> {
    if state.ui_flags.ask_for_confirmation
        && !state.ui_flags.show_confirmation_popup
    {
        state.ui_flags.show_confirmation_popup = true;
        return Ok(());
    }

    state.ui_flags.show_confirmation_popup = false;

    let Some((idx, selection)) = state.items.get_selected_item() else {
        return Ok(());
    };

    if selection.saved {
        actions::delete(&selection.name)?;
        state.items.update_item(&selection.name, Some(false), None);
    } else {
        tmux::interface::close_session(&selection.name)?;
        state.items.update_item(&selection.name, None, Some(false));
    }

    if (selection.saved && !selection.active)
        || (!selection.saved && selection.active)
    {
        state.items.remove_item(idx, selection);
    }

    state.items.update_filter();

    Ok(())
}

fn handle_edit(
    state: &mut MenuState,
    terminal: &mut DefaultTerminal,
) -> Result<()> {
    let Some((_, selection)) = state.items.get_selected_item() else {
        return Ok(());
    };

    if selection.saved {
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)?;

        actions::edit(Some(&selection.name))?;

        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        terminal.clear()?;
    }

    Ok(())
}

fn handle_save(state: &mut MenuState) -> Result<()> {
    let Some((_, selection)) = state.items.get_selected_item() else {
        return Ok(());
    };

    if !selection.saved {
        actions::save_target(&selection.name)?;
        state.items.update_item(&selection.name, Some(true), None);
        state.items.update_filter();
    }

    Ok(())
}

fn handle_kill(state: &mut MenuState) -> Result<()> {
    let Some((idx, selection)) = state.items.get_selected_item() else {
        return Ok(());
    };

    if selection.active {
        tmux::interface::close_session(&selection.name)?;
        state.items.update_item(&selection.name, None, Some(false));

        if !selection.saved {
            state.items.remove_item(idx, selection);
        }

        state.items.update_filter();
    }

    Ok(())
}

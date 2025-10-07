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

use crate::{actions, menu::state::MenuState, tmux};
use crate::{
    menu::{action::MenuAction, state::MenuMode},
    util::validate_session_name,
};

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
            MenuAction::Open => handle_open(state)?,
            MenuAction::Delete => handle_delete(state)?,
            MenuAction::Edit => handle_edit(state, terminal)?,
            MenuAction::Save => handle_save(state)?,
            MenuAction::Rename => handle_rename(state)?,
            MenuAction::Kill => handle_kill(state)?,
            MenuAction::MoveSelection(delta) => {
                state.items.move_selection(delta)
            }
            MenuAction::RemoveLastWord => {
                state.handle_textarea_input(|t| {
                    t.delete_word();
                });
            }
            MenuAction::AppendToInput(c) => {
                state.handle_textarea_input(|t| {
                    t.insert_char(c);
                });
            }
            MenuAction::DeleteFromInput => {
                state.handle_textarea_input(|t| {
                    t.delete_char();
                });
            }
            MenuAction::TogglePreview => {
                state.ui_flags.show_preview = !state.ui_flags.show_preview;
            }
            MenuAction::ToggleHelp => {
                if state.mode == MenuMode::HelpPopup {
                    state.mode = MenuMode::Normal;
                } else if state.mode == MenuMode::Normal {
                    state.mode = MenuMode::HelpPopup;
                }
            }
            MenuAction::HideConfirmation => {
                state.mode = MenuMode::Normal;
            }
            MenuAction::EnterRenameMode => handle_enter_rename(state)?,
            MenuAction::ExitRenameMode => state.mode = MenuMode::Normal,
            MenuAction::CloseErrorPopup => state.mode = MenuMode::Normal,
            MenuAction::Exit => {
                state.should_exit = true;
            }
            MenuAction::Nop => {}
        };

        Ok(())
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
    if state.ui_flags.ask_for_confirmation && state.mode == MenuMode::Normal {
        state.mode = MenuMode::ConfirmationPopup;
        return Ok(());
    }

    state.mode = MenuMode::Normal;

    let Some((idx, selection)) = state.items.get_selected_item() else {
        return Ok(());
    };

    if selection.saved {
        actions::delete(&selection.name)?;
        state
            .items
            .update_item(&selection.name, Some(false), None, None);
    } else {
        tmux::interface::close_session(&selection.name)?;
        state
            .items
            .update_item(&selection.name, None, Some(false), None);
    }

    if (selection.saved && !selection.active)
        || (!selection.saved && selection.active)
    {
        state.items.remove_item(idx, selection);
    }

    state
        .items
        .update_filter(&state.filter_input.lines().join("\n"));

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
        state
            .items
            .update_item(&selection.name, Some(true), None, None);
        state
            .items
            .update_filter(&state.filter_input.lines().join("\n"));
    }

    Ok(())
}

fn handle_rename(state: &mut MenuState) -> Result<()> {
    let Some((_, selection)) = state.items.get_selected_item() else {
        return Ok(());
    };

    state.mode = MenuMode::Normal;

    let new_name = state.rename_input.lines().join("\n");

    if let Err(err) = validate_session_name(&new_name) {
        state.mode = MenuMode::ErrorPopup(err.to_string());
        return Ok(());
    }

    state
        .items
        .update_item(&selection.name, None, None, Some(&new_name));

    if selection.active {
        tmux::interface::rename_session(&selection.name, &new_name)?;
    }

    if selection.saved {
        actions::rename(&selection.name, &new_name)?;
    }

    Ok(())
}

fn handle_kill(state: &mut MenuState) -> Result<()> {
    let Some((idx, selection)) = state.items.get_selected_item() else {
        return Ok(());
    };

    if selection.active {
        tmux::interface::close_session(&selection.name)?;
        state
            .items
            .update_item(&selection.name, None, Some(false), None);

        if !selection.saved {
            state.items.remove_item(idx, selection);
        }

        state
            .items
            .update_filter(&state.filter_input.lines().join("\n"));
    }

    Ok(())
}

fn handle_enter_rename(state: &mut MenuState) -> Result<()> {
    state.mode = MenuMode::Rename;

    state.rename_input.delete_line_by_head();

    let placeholder;
    if let Some((_, menu_item)) = state.items.get_selected_item() {
        placeholder = menu_item.name;
    } else {
        placeholder = String::new();
    }
    state.rename_input.insert_str(placeholder);

    Ok(())
}

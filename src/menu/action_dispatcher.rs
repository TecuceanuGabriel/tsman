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
    menu::{
        action::MenuAction,
        item::MenuItem,
        state::{ListMode, MenuMode},
    },
    persistence::{self, StorageKind},
    util::validate_session_name,
};

/// Executes a [`MenuAction`] by mutating state and calling tmux/persistence APIs.
pub trait ActionDispatcher {
    fn dispach(
        &self,
        action: MenuAction,
        state: &mut MenuState,
        terminal: &mut DefaultTerminal,
    ) -> Result<()>;
}

/// Default action dispatcher that handles all [`MenuAction`] variants.
pub struct DefaultActionDispacher;

impl ActionDispatcher for DefaultActionDispacher {
    fn dispach(
        &self,
        action: MenuAction,
        state: &mut MenuState,
        terminal: &mut DefaultTerminal,
    ) -> Result<()> {
        match action {
            MenuAction::Open => {
                if state.list_mode == ListMode::Layouts {
                    handle_enter_create_name(state)?;
                } else {
                    handle_open(state)?;
                }
            }
            MenuAction::Delete => handle_delete(state)?,
            MenuAction::Edit => handle_edit(state, terminal)?,
            MenuAction::Save => handle_save(state)?,
            MenuAction::Rename => handle_rename(state)?,
            MenuAction::Kill => handle_kill(state)?,
            MenuAction::Reload => handle_reload(state)?,
            MenuAction::MoveSelection(delta) => {
                state.items.move_selection(delta);
                state.preview_scroll = 0;
            }
            MenuAction::RemoveLastWord => {
                state.handle_textarea_input(|t| {
                    t.delete_word();
                });
            }
            MenuAction::DeleteToLineStart => {
                state.handle_textarea_input(|t| {
                    t.delete_line_by_head();
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
            MenuAction::ScrollPreviewDown => {
                state.preview_scroll = state.preview_scroll.saturating_add(1);
            }
            MenuAction::ScrollPreviewUp => {
                state.preview_scroll = state.preview_scroll.saturating_sub(1);
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
            MenuAction::ToggleListMode => handle_toggle_list_mode(state)?,
            MenuAction::ConfirmCreateName => handle_confirm_create_name(state)?,
            MenuAction::CreateFromLayout => handle_create_from_layout(state)?,
            MenuAction::ExitCreateMode => handle_exit_create_mode(state),
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

        state.items.sort();
        state
            .items
            .update_filter(&state.filter_input.lines().join("\n"));
    }

    Ok(())
}

fn handle_reload(state: &mut MenuState) -> Result<()> {
    if state.list_mode != ListMode::Sessions {
        return Ok(());
    }

    let Some((_, selection)) = state.items.get_selected_item() else {
        return Ok(());
    };

    if !selection.saved || !selection.active {
        state.mode = MenuMode::ErrorPopup(
            "Session must be both active and saved to reload".to_string(),
        );
        return Ok(());
    }

    match actions::reload(Some(&selection.name)) {
        Ok(()) => {
            state.should_exit = true;
        }
        Err(err) => {
            state.mode = MenuMode::ErrorPopup(err.to_string());
        }
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

fn handle_toggle_list_mode(state: &mut MenuState) -> Result<()> {
    state.list_mode = match state.list_mode {
        ListMode::Sessions => ListMode::Layouts,
        ListMode::Layouts => ListMode::Sessions,
    };

    let items = match state.list_mode {
        ListMode::Sessions => {
            let saved: std::collections::HashSet<String> =
                persistence::list_saved_configs(StorageKind::Session)?
                    .into_iter()
                    .collect();
            let active: std::collections::HashSet<String> =
                tmux::interface::list_active_sessions()?
                    .into_iter()
                    .collect();
            let union: std::collections::HashSet<_> =
                saved.union(&active).cloned().collect();
            union
                .into_iter()
                .map(|name| {
                    MenuItem::new(
                        name.clone(),
                        saved.contains(&name),
                        active.contains(&name),
                    )
                })
                .collect()
        }
        ListMode::Layouts => {
            persistence::list_saved_configs(StorageKind::Layout)?
                .into_iter()
                .map(|name| MenuItem::new(name, true, false))
                .collect()
        }
    };

    state.items.replace_items(items);
    state.filter_input.delete_line_by_head();

    Ok(())
}

fn handle_enter_create_name(state: &mut MenuState) -> Result<()> {
    if state.items.get_selected_item().is_none() {
        return Ok(());
    }

    state.mode = MenuMode::CreateFromLayoutName;
    state.rename_input.delete_line_by_head();

    Ok(())
}

fn handle_confirm_create_name(state: &mut MenuState) -> Result<()> {
    let name = state.rename_input.lines().join("\n");

    if let Err(err) = validate_session_name(&name) {
        state.mode = MenuMode::ErrorPopup(err.to_string());
        return Ok(());
    }

    state.pending_create_name = name;
    state.mode = MenuMode::CreateFromLayoutWorkdir;
    state.rename_input.delete_line_by_head();

    Ok(())
}

fn handle_create_from_layout(state: &mut MenuState) -> Result<()> {
    let work_dir = state.rename_input.lines().join("\n");

    let Some((_, selection)) = state.items.get_selected_item() else {
        return Ok(());
    };

    let session_name = state.pending_create_name.clone();

    match actions::layout_create(
        &selection.name,
        &work_dir,
        Some(&session_name),
    ) {
        Ok(()) => {
            state.should_exit = true;
        }
        Err(err) => {
            state.mode = MenuMode::ErrorPopup(err.to_string());
        }
    }

    Ok(())
}

fn handle_exit_create_mode(state: &mut MenuState) {
    state.mode = MenuMode::Normal;
    state.rename_input.delete_line_by_head();
}

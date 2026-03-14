use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::menu::{
    action::MenuAction,
    state::{MenuMode, MenuState},
};

/// Maps terminal events to [`MenuAction`]s based on the current mode.
pub trait EventHandler {
    fn handle_event(
        &self,
        event: Event,
        state: &MenuState,
    ) -> (MenuAction, Option<String>);
}

/// Default keyboard-driven event handler.
pub struct DefaultEventHandler;

impl EventHandler for DefaultEventHandler {
    fn handle_event(
        &self,
        event: Event,
        state: &MenuState,
    ) -> (MenuAction, Option<String>) {
        let Event::Key(key) = event else {
            return (MenuAction::Nop, None);
        };

        if key.kind != KeyEventKind::Press {
            return (MenuAction::Nop, None);
        }

        let action = match state.mode {
            MenuMode::Normal => handle_normal_mode_key(key),
            MenuMode::Rename => handle_rename_mode_key(key),
            MenuMode::HelpPopup => handle_help_popup_key(key),
            MenuMode::ConfirmationPopup => handle_confirmation_popup_key(key),
            MenuMode::ErrorPopup(_) => handle_error_popup_key(key),
            MenuMode::CreateFromLayoutName => handle_create_name_mode_key(key),
            MenuMode::CreateFromLayoutWorkdir => {
                handle_create_workdir_mode_key(key)
            }
        };

        let label = key_event_to_label(key);
        (action, label)
    }
}

fn handle_normal_mode_key(key: KeyEvent) -> MenuAction {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);

    match (ctrl, shift, key.code) {
        (true, _, KeyCode::Char('p')) => MenuAction::MoveSelection(-1),
        (true, _, KeyCode::Char('n')) => MenuAction::MoveSelection(1),
        (true, _, KeyCode::Char('r')) => MenuAction::EnterRenameMode,
        (true, _, KeyCode::Char('e')) => MenuAction::Edit,
        (true, _, KeyCode::Char('s')) => MenuAction::Save,
        (true, _, KeyCode::Char('d')) => MenuAction::Delete,
        (true, _, KeyCode::Char('k')) => MenuAction::Kill,
        (true, _, KeyCode::Char('o')) => MenuAction::Reload,
        (true, _, KeyCode::Char('c')) => MenuAction::Exit,
        (true, _, KeyCode::Char('l')) => MenuAction::ToggleListMode,
        (true, _, KeyCode::Char('t')) => MenuAction::TogglePreview,
        (true, _, KeyCode::Char('h')) => MenuAction::ToggleHelp,
        (true, _, KeyCode::Char('w')) => MenuAction::RemoveLastWord,

        (false, true, KeyCode::Up) => MenuAction::ScrollPreviewUp,
        (false, true, KeyCode::Down) => MenuAction::ScrollPreviewDown,

        (false, _, KeyCode::Char(c)) => MenuAction::AppendToInput(c),
        (false, _, KeyCode::Backspace) => MenuAction::DeleteFromInput,
        (false, _, KeyCode::Up) => MenuAction::MoveSelection(-1),
        (false, _, KeyCode::Down) => MenuAction::MoveSelection(1),
        (false, _, KeyCode::Enter) => MenuAction::Open,
        (false, _, KeyCode::Esc) => MenuAction::Exit,

        _ => MenuAction::Nop,
    }
}

fn handle_rename_mode_key(key: KeyEvent) -> MenuAction {
    match (key.modifiers.contains(KeyModifiers::CONTROL), key.code) {
        (true, KeyCode::Char('c')) => MenuAction::ExitRenameMode,
        (true, KeyCode::Char('w')) => MenuAction::RemoveLastWord,

        (false, KeyCode::Char(c)) => MenuAction::AppendToInput(c),
        (false, KeyCode::Backspace) => MenuAction::DeleteFromInput,
        (false, KeyCode::Enter) => MenuAction::Rename,
        (false, KeyCode::Esc) => MenuAction::ExitRenameMode,

        _ => MenuAction::Nop,
    }
}

fn handle_confirmation_popup_key(key: KeyEvent) -> MenuAction {
    match key.code {
        KeyCode::Char('y' | 'Y') | KeyCode::Enter => MenuAction::Delete,
        KeyCode::Char('n' | 'N' | 'q') | KeyCode::Esc => {
            MenuAction::HideConfirmation
        }
        _ => MenuAction::Nop,
    }
}

fn handle_help_popup_key(key: KeyEvent) -> MenuAction {
    match (key.modifiers.contains(KeyModifiers::CONTROL), key.code) {
        (true, KeyCode::Char('h' | 'c')) => MenuAction::ToggleHelp,
        (false, KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter) => {
            MenuAction::ToggleHelp
        }
        _ => MenuAction::Nop,
    }
}

fn handle_error_popup_key(_key: KeyEvent) -> MenuAction {
    MenuAction::CloseErrorPopup
}

fn handle_create_name_mode_key(key: KeyEvent) -> MenuAction {
    match (key.modifiers.contains(KeyModifiers::CONTROL), key.code) {
        (true, KeyCode::Char('c')) => MenuAction::ExitCreateMode,
        (true, KeyCode::Char('w')) => MenuAction::RemoveLastWord,

        (false, KeyCode::Char(c)) => MenuAction::AppendToInput(c),
        (false, KeyCode::Backspace) => MenuAction::DeleteFromInput,
        (false, KeyCode::Enter) => MenuAction::ConfirmCreateName,
        (false, KeyCode::Esc) => MenuAction::ExitCreateMode,

        _ => MenuAction::Nop,
    }
}

fn handle_create_workdir_mode_key(key: KeyEvent) -> MenuAction {
    match (key.modifiers.contains(KeyModifiers::CONTROL), key.code) {
        (true, KeyCode::Char('c')) => MenuAction::ExitCreateMode,
        (true, KeyCode::Char('w')) => MenuAction::RemoveLastWord,

        (false, KeyCode::Char(c)) => MenuAction::AppendToInput(c),
        (false, KeyCode::Backspace) => MenuAction::DeleteFromInput,
        (false, KeyCode::Enter) => MenuAction::CreateFromLayout,
        (false, KeyCode::Esc) => MenuAction::ExitCreateMode,

        _ => MenuAction::Nop,
    }
}

/// Converts a key event into a human-readable label for display.
/// Returns `None` for plain character keys to avoid cluttering the indicator.
fn key_event_to_label(key: KeyEvent) -> Option<String> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);

    match (ctrl, shift, key.code) {
        (true, _, KeyCode::Char(c)) => Some(format!("C-{c}")),
        (_, true, KeyCode::Up) => Some("S-Up".into()),
        (_, true, KeyCode::Down) => Some("S-Down".into()),
        (false, false, KeyCode::Enter) => Some("Enter".into()),
        (false, false, KeyCode::Esc) => Some("Esc".into()),
        (false, false, KeyCode::Backspace) => Some("Bksp".into()),
        (false, false, KeyCode::Up) => Some("Up".into()),
        (false, false, KeyCode::Down) => Some("Down".into()),
        _ => None,
    }
}

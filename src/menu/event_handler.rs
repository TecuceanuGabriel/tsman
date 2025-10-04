use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::menu::{
    action::MenuAction,
    state::{MenuMode, MenuState},
};

pub trait EventHandler {
    fn handle_event(&self, event: Event, state: &MenuState) -> MenuAction;
}

pub struct DefaultEventHandler;

impl EventHandler for DefaultEventHandler {
    fn handle_event(&self, event: Event, state: &MenuState) -> MenuAction {
        let Event::Key(key) = event else {
            return MenuAction::Nop;
        };

        if key.kind != KeyEventKind::Press {
            return MenuAction::Nop;
        }

        match state.mode {
            MenuMode::Normal => handle_normal_mode_key(key),
            MenuMode::Rename => handle_rename_mode_key(key),
            MenuMode::HelpPopup => handle_help_popup_key(key),
            MenuMode::ConfirmationPopup => handle_confirmation_popup_key(key),
        }
    }
}

fn handle_normal_mode_key(key: KeyEvent) -> MenuAction {
    match (key.modifiers.contains(KeyModifiers::CONTROL), key.code) {
        (true, KeyCode::Char('p')) => MenuAction::MoveSelection(-1),
        (true, KeyCode::Char('n')) => MenuAction::MoveSelection(1),
        (true, KeyCode::Char('r')) => MenuAction::EnterRenameMode,
        (true, KeyCode::Char('e')) => MenuAction::Edit,
        (true, KeyCode::Char('s')) => MenuAction::Save,
        (true, KeyCode::Char('d')) => MenuAction::Delete,
        (true, KeyCode::Char('k')) => MenuAction::Kill,
        (true, KeyCode::Char('c')) => MenuAction::Exit,
        (true, KeyCode::Char('t')) => MenuAction::TogglePreview,
        (true, KeyCode::Char('h')) => MenuAction::ToggleHelp,
        (true, KeyCode::Char('w')) => MenuAction::RemoveLastWord,

        (false, KeyCode::Char(c)) => MenuAction::AppendToInput(c),
        (false, KeyCode::Backspace) => MenuAction::DeleteFromInput,
        (false, KeyCode::Up) => MenuAction::MoveSelection(-1),
        (false, KeyCode::Down) => MenuAction::MoveSelection(1),
        (false, KeyCode::Enter) => MenuAction::Open,
        (false, KeyCode::Esc) => MenuAction::Exit,

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

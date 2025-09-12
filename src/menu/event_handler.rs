use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::menu::{action::MenuAction, state::MenuState};

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

        if state.ui_flags.show_confirmation_popup {
            return handle_confirmation_popup_key(key);
        }

        if state.ui_flags.show_help {
            return handle_help_popup_key(key);
        }

        handle_normal_mode_key(key)
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

fn handle_normal_mode_key(key: KeyEvent) -> MenuAction {
    match (key.modifiers.contains(KeyModifiers::CONTROL), key.code) {
        (true, KeyCode::Char('p')) => MenuAction::MoveSelection(-1),
        (true, KeyCode::Char('n')) => MenuAction::MoveSelection(1),
        (true, KeyCode::Char('e')) => MenuAction::Edit,
        (true, KeyCode::Char('s')) => MenuAction::Save,
        (true, KeyCode::Char('d')) => MenuAction::Delete,
        (true, KeyCode::Char('k')) => MenuAction::Kill,
        (true, KeyCode::Char('r')) => MenuAction::Reload,
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

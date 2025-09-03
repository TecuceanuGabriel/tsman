use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::menu::{menu_action::MenuAction, menu_state::MenuState};

pub trait EventHandler {
    fn handle_event(&mut self, event: Event, state: &MenuState) -> MenuAction;
}

pub struct DefaultEventHandler;

impl EventHandler for DefaultEventHandler {
    fn handle_event(&mut self, event: Event, state: &MenuState) -> MenuAction {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return MenuAction::NOP;
            }

            if state.ui_flags.show_confirmation_popup {
                return handle_confirmation_popup_key(key);
            }

            if state.ui_flags.show_help {
                return handle_help_popup_key(key);
            }

            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return handle_modifier_key_combo(key);
            } else {
                return handle_regular_key(key);
            }
        }

        return MenuAction::NOP;
    }
}

fn handle_confirmation_popup_key(key: KeyEvent) -> MenuAction {
    match key.code {
        KeyCode::Char('y' | 'Y') | KeyCode::Enter => return MenuAction::Delete,
        KeyCode::Char('n' | 'N' | 'q') | KeyCode::Esc => {
            return MenuAction::HideConfirmation;
        }
        _ => return MenuAction::NOP,
    }
}

fn handle_help_popup_key(key: KeyEvent) -> MenuAction {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char('h' | 'c') = key.code {
            return MenuAction::ToggleHelp;
        }
    } else {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => {
                return MenuAction::ToggleHelp;
            }
            _ => {}
        }
    }

    return MenuAction::NOP;
}

fn handle_modifier_key_combo(key: KeyEvent) -> MenuAction {
    match key.code {
        KeyCode::Char('p') => MenuAction::MoveSelection(-1),
        KeyCode::Char('n') => MenuAction::MoveSelection(1),
        KeyCode::Char('e') => MenuAction::Edit,
        KeyCode::Char('s') => MenuAction::Save,
        KeyCode::Char('d') => MenuAction::Delete,
        KeyCode::Char('k') => MenuAction::Kill,
        KeyCode::Char('c') => MenuAction::Exit,
        KeyCode::Char('t') => MenuAction::TogglePreview,
        KeyCode::Char('h') => MenuAction::ToggleHelp,
        KeyCode::Char('w') => MenuAction::RemoveLastWord,
        _ => MenuAction::NOP,
    }
}

fn handle_regular_key(key: KeyEvent) -> MenuAction {
    match key.code {
        KeyCode::Char(c) => MenuAction::AppendToInput(c),
        KeyCode::Backspace => MenuAction::DeleteFromInput,
        KeyCode::Up => MenuAction::MoveSelection(-1),
        KeyCode::Down => MenuAction::MoveSelection(1),
        KeyCode::Enter => MenuAction::Open,
        KeyCode::Esc => MenuAction::Exit,
        _ => MenuAction::NOP,
    }
}

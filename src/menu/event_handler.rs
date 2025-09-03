use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::menu::menu_state::MenuState;

pub trait EventHandler {
    fn handle_event(&mut self, event: Event, state: &MenuState);
}

pub struct DefaultEventHanlder;

impl EventHandler for DefaultEventHanlder {
    fn handle_event(&mut self, event: Event, state: &MenuState) {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return;
            }

            if state.ui_flags.show_confirmation_popup {
                handle_confirmation_popup_key(key);
                return;
            }

            if state.ui_flags.show_help {
                handle_help_popup_key(key);
                return;
            }

            if key.modifiers.contains(KeyModifiers::CONTROL) {
                handle_modifier_key_combo(key);
            } else {
                handle_regular_key(key);
            }
        }
    }
}

fn handle_confirmation_popup_key(key: KeyEvent) {
    match key.code {
        KeyCode::Char('y' | 'Y') | KeyCode::Enter => {
            self.handle_delete()?;
            self.state.ui_flags.show_confirmation_popup = false;
        }
        KeyCode::Char('n' | 'N' | 'q') | KeyCode::Esc => {
            self.state.ui_flags.show_confirmation_popup = false;
        }
        _ => {}
    }

    Ok(())
}

fn handle_help_popup_key(key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char('h' | 'c') = key.code {
            self.state.ui_flags.show_help = !self.state.ui_flags.show_help
        }
    } else {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => {
                self.state.ui_flags.show_help = !self.state.ui_flags.show_help
            }
            _ => {}
        }
    }

    Ok(())
}

fn handle_modifier_key_combo(key: KeyEvent) {
    match key.code {
        KeyCode::Char('p') => self.state.items.move_selection(-1),
        KeyCode::Char('n') => self.state.items.move_selection(1),
        KeyCode::Char('e') => self.handle_edit(terminal)?,
        KeyCode::Char('s') => self.handle_save()?,
        KeyCode::Char('d') => {
            if self.state.ui_flags.ask_for_confirmation {
                self.state.ui_flags.show_confirmation_popup = true;
            } else {
                self.handle_delete()?;
            }
        }
        KeyCode::Char('k') => self.handle_kill()?,
        KeyCode::Char('c') => self.state.should_exit = true,
        KeyCode::Char('t') => {
            self.state.ui_flags.show_preview = !self.state.ui_flags.show_preview
        }
        KeyCode::Char('h') => {
            self.state.ui_flags.show_help = !self.state.ui_flags.show_help
        }
        KeyCode::Char('w') => {
            self.state.items.remove_last_word_from_input();
        }
        _ => {}
    }
    Ok(())
}

fn handle_regular_key(key: KeyEvent) {
    match key.code {
        KeyCode::Char(c) => {
            self.state.items.input.push(c);
            self.state.items.update_filter_and_reset();
        }
        KeyCode::Backspace => {
            self.state.items.input.pop();
            self.state.items.update_filter_and_reset();
        }
        KeyCode::Up => self.state.items.move_selection(-1),
        KeyCode::Down => self.state.items.move_selection(1),
        KeyCode::Enter => self.handle_open()?,
        KeyCode::Esc => self.state.should_exit = true,
        _ => {}
    }

    Ok(())
}

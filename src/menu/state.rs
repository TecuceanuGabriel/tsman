use ratatui::style::Style;
use tui_textarea::TextArea;

use crate::menu::{item::MenuItem, items_state::ItemsState, ui_flags::UiFlags};

#[derive(PartialEq)]
pub enum MenuMode {
    Normal,
    Rename,
    HelpPopup,
    ConfirmationPopup,
    ErrorPopup(String),
}

pub struct MenuState<'a> {
    pub filter_input: TextArea<'a>,
    pub rename_input: TextArea<'a>,
    pub items: ItemsState,

    pub mode: MenuMode,
    pub ui_flags: UiFlags,

    pub should_exit: bool,
}

impl<'a> MenuState<'a> {
    pub fn new(
        items: Vec<MenuItem>,
        show_preview: bool,
        ask_for_confirmation: bool,
    ) -> Self {
        let mut filter_input = TextArea::default();
        filter_input.set_cursor_line_style(Style::default());

        let mut rename_input = TextArea::default();
        rename_input.set_cursor_line_style(Style::default());

        Self {
            filter_input,
            rename_input,
            items: ItemsState::new(items),
            mode: MenuMode::Normal,
            ui_flags: UiFlags::new(show_preview, ask_for_confirmation),
            should_exit: false,
        }
    }

    pub fn get_active_textarea(&mut self) -> &mut TextArea<'a> {
        match self.mode {
            MenuMode::Rename => &mut self.rename_input,
            _ => &mut self.filter_input,
        }
    }

    pub fn handle_textarea_input<F>(&mut self, operation: F)
    where
        F: FnOnce(&mut TextArea),
    {
        let textarea = self.get_active_textarea();
        operation(textarea);

        let text = textarea.lines().join("\n");
        if self.mode == MenuMode::Normal {
            self.items.update_filter_and_reset(&text);
        }
    }
}

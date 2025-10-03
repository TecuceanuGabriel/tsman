use ratatui::style::Style;
use tui_textarea::TextArea;

use crate::menu::{item::MenuItem, items_state::ItemsState, ui_flags::UiFlags};

#[derive(PartialEq)]
pub enum MenuMode {
    Normal,
    HelpPopup,
    ConfirmationPopup,
}

pub struct MenuState<'a> {
    pub input: TextArea<'a>,
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
        let mut input = TextArea::default();
        input.set_cursor_line_style(Style::default());

        Self {
            input,
            items: ItemsState::new(items),
            mode: MenuMode::Normal,
            ui_flags: UiFlags::new(show_preview, ask_for_confirmation),
            should_exit: false,
        }
    }
}

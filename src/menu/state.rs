use ratatui::style::Style;
use tui_textarea::TextArea;

use crate::menu::{item::MenuItem, items_state::ItemsState, ui_flags::UiFlags};

/// Whether the menu is showing sessions or layouts.
#[derive(PartialEq)]
pub enum ListMode {
    Sessions,
    Layouts,
}

/// Current interaction mode of the menu.
#[derive(PartialEq)]
pub enum MenuMode {
    Normal,
    Rename,
    HelpPopup,
    ConfirmationPopup,
    ErrorPopup(String),
    CreateFromLayoutName,
    CreateFromLayoutWorkdir,
}

/// All mutable state for the menu UI.
pub struct MenuState<'a> {
    pub filter_input: TextArea<'a>,
    pub rename_input: TextArea<'a>,
    pub items: ItemsState,

    pub mode: MenuMode,
    pub list_mode: ListMode,
    pub pending_create_name: String,
    pub ui_flags: UiFlags,
    pub preview_scroll: u16,

    pub should_exit: bool,
}

impl<'a> MenuState<'a> {
    /// Creates initial menu state from the given items and flags.
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
            list_mode: ListMode::Sessions,
            pending_create_name: String::new(),
            ui_flags: UiFlags::new(show_preview, ask_for_confirmation),
            preview_scroll: 0,
            should_exit: false,
        }
    }

    /// Returns the textarea active for the current mode (rename or filter).
    pub fn get_active_textarea(&mut self) -> &mut TextArea<'a> {
        match self.mode {
            MenuMode::Rename
            | MenuMode::CreateFromLayoutName
            | MenuMode::CreateFromLayoutWorkdir => &mut self.rename_input,
            _ => &mut self.filter_input,
        }
    }

    /// Applies an edit operation to the active textarea and updates the filter if in normal mode.
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

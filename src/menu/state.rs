use crate::menu::{item::MenuItem, items_state::ItemsState, ui_flags::UiFlags};

pub struct MenuState {
    pub items: ItemsState,
    pub ui_flags: UiFlags,
    pub should_exit: bool,
}

impl MenuState {
    pub fn new(
        items: Vec<MenuItem>,
        show_preview: bool,
        ask_for_confirmation: bool,
    ) -> Self {
        Self {
            items: ItemsState::new(items),
            ui_flags: UiFlags::new(show_preview, ask_for_confirmation),
            should_exit: false,
        }
    }
}

use ratatui::widgets::ListState;

use crate::menu::item::MenuItem;

pub struct ItemsState {
    pub all_items: Vec<MenuItem>,
    pub filtered_items: Vec<MenuItem>,
    pub input: String,
    pub list_state: ListState,
}

impl ItemsState {
    pub fn new(items: Vec<MenuItem>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            all_items: items.clone(),
            filtered_items: items,
            input: String::new(),
            list_state,
        }
    }
}

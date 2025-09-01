use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use ratatui::widgets::ListState;

use crate::menu::item::MenuItem;

pub struct ItemsState {
    pub all_items: Vec<MenuItem>,
    pub filtered_items: Vec<MenuItem>,
    pub input: String,
    pub list_state: ListState,

    matcher: SkimMatcherV2,
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
            matcher: fuzzy_matcher::skim::SkimMatcherV2::default(),
        }
    }

    pub fn update_item(
        &mut self,
        name: &str,
        saved: Option<bool>,
        active: Option<bool>,
    ) {
        if let Some(item) = self.all_items.iter_mut().find(|i| i.name == name) {
            if let Some(saved_val) = saved {
                item.saved = saved_val;
            }
            if let Some(active_val) = active {
                item.active = active_val;
            }
        }
    }

    pub fn move_selection(&mut self, delta: i32) {
        if let Some(selection_idx) = self.list_state.selected() {
            let new_selected =
                usize::try_from((selection_idx as i32 + delta).max(0))
                    .unwrap_or(0);
            self.list_state.select(Some(
                new_selected.min(self.filtered_items.len().saturating_sub(1)),
            ));
        }
    }

    pub fn remove_last_word_from_input(&mut self) {
        if self.input.is_empty() {
            return;
        }

        if let Some(last_space) = self.input.trim_end().rfind(' ') {
            self.input.truncate(last_space);
        } else {
            self.input.clear();
        }

        self.update_filter_and_reset();
    }

    pub fn update_filter_and_reset(&mut self) {
        self.update_filter();
        self.reset_position();
    }

    pub fn update_filter(&mut self) {
        if self.input.is_empty() {
            self.filtered_items = self.all_items.clone();
        } else {
            self.filtered_items = self
                .all_items
                .iter()
                .filter(|item| {
                    self.matcher.fuzzy_match(&item.name, &self.input).is_some()
                })
                .cloned()
                .collect();
        }
    }

    fn reset_position(&mut self) {
        if self.filtered_items.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }
}

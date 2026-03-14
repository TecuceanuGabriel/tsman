use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use ratatui::widgets::ListState;

use crate::menu::item::MenuItem;

/// Manages the item list, fuzzy filtering, and selection cursor.
pub struct ItemsState {
    pub items: Vec<MenuItem>,
    pub filtered_items_idx: Vec<(usize, Vec<usize>)>,
    pub list_state: ListState,

    matcher: SkimMatcherV2,
}

impl ItemsState {
    /// Creates a new state, sorting items and selecting the first one.
    pub fn new(mut items: Vec<MenuItem>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        sort_items(&mut items);

        let mut state = Self {
            filtered_items_idx: (0..items.len())
                .map(|i| (i, Vec::new()))
                .collect(),
            items,
            list_state,
            matcher: fuzzy_matcher::skim::SkimMatcherV2::default(),
        };

        state.update_filter("");

        state
    }

    /// Returns the selected item's filtered index and a clone of it.
    pub fn get_selected_item(&self) -> Option<(usize, MenuItem)> {
        let idx = self.list_state.selected()?;
        let &(item_idx, _) = self.filtered_items_idx.get(idx)?;
        let item = self.items.get(item_idx)?.clone();
        Some((idx, item))
    }

    /// Returns references to items and their fuzzy match indices.
    pub fn get_filtered_items(&self) -> Vec<(&MenuItem, &[usize])> {
        self.filtered_items_idx
            .iter()
            .map(|(idx, indices)| {
                (self.items.get(*idx).unwrap(), indices.as_slice())
            })
            .collect()
    }

    /// Updates fields of the item matching `name`. `None` fields are left unchanged.
    pub fn update_item(
        &mut self,
        name: &str,
        saved: Option<bool>,
        active: Option<bool>,
        new_name: Option<&str>,
    ) {
        if let Some(item) = self.items.iter_mut().find(|i| i.name == name) {
            if let Some(saved_val) = saved {
                item.saved = saved_val;
            }
            if let Some(active_val) = active {
                item.active = active_val;
            }
            if let Some(name) = new_name {
                item.name = name.to_owned();
            }
        }
    }

    /// Re-sorts items by active status and name.
    pub fn sort(&mut self) {
        sort_items(&mut self.items);
    }

    /// Moves the selection cursor by `delta`, clamped to list bounds.
    pub fn move_selection(&mut self, delta: i32) {
        if let Some(selection_idx) = self.list_state.selected() {
            let new_selected =
                usize::try_from((selection_idx as i32 + delta).max(0))
                    .unwrap_or(0);
            self.list_state.select(Some(
                new_selected
                    .min(self.filtered_items_idx.len().saturating_sub(1)),
            ));
        }
    }

    /// Removes an item by name and adjusts the selection.
    pub fn remove_item(&mut self, idx: usize, item: MenuItem) {
        self.items.retain(|i| i.name != item.name);
        self.list_state.select(Some(idx.saturating_sub(1)));
    }

    /// Replaces the entire item list, resetting filter and selection.
    pub fn replace_items(&mut self, mut items: Vec<MenuItem>) {
        sort_items(&mut items);
        self.items = items;
        self.filtered_items_idx =
            (0..self.items.len()).map(|i| (i, Vec::new())).collect();
        self.reset_position();
    }

    /// Re-filters items and resets the selection to the top.
    pub fn update_filter_and_reset(&mut self, input: &str) {
        self.update_filter(input);
        self.reset_position();
    }

    /// Re-filters items by fuzzy-matching against `input`, keeping the current selection.
    pub fn update_filter(&mut self, input: &str) {
        if input.is_empty() {
            self.filtered_items_idx =
                (0..self.items.len()).map(|i| (i, Vec::new())).collect();
        } else {
            self.filtered_items_idx = self
                .items
                .iter()
                .enumerate()
                .filter_map(|(idx, item)| {
                    self.matcher
                        .fuzzy_indices(&item.name, input)
                        .map(|(_, indices)| (idx, indices))
                })
                .collect();
        }
    }

    fn reset_position(&mut self) {
        if self.filtered_items_idx.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }
}

fn sort_items(items: &mut [MenuItem]) {
    items.sort_by(|a, b| b.active.cmp(&a.active).then(a.name.cmp(&b.name)))
}

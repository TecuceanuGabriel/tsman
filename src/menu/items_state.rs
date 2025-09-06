use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use ratatui::widgets::ListState;

use crate::menu::item::MenuItem;

pub struct ItemsState {
    pub items: Vec<MenuItem>,
    pub filtered_items_idx: Vec<usize>,
    pub list_state: ListState,

    matcher: SkimMatcherV2,
}

impl ItemsState {
    pub fn new(items: Vec<MenuItem>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        let all_items = sort_items(&items);

        let mut state = Self {
            items: all_items,
            filtered_items_idx: (0..items.len()).collect(),
            list_state,
            matcher: fuzzy_matcher::skim::SkimMatcherV2::default(),
        };

        state.update_filter("");

        return state;
    }

    pub fn get_selected_item(&self) -> Option<(usize, MenuItem)> {
        let idx = self.list_state.selected()?;
        let &item_idx = self.filtered_items_idx.get(idx)?;
        let item = self.items.get(item_idx)?.clone();
        Some((idx, item))
    }

    pub fn get_filtered_items(&self) -> Vec<&MenuItem> {
        self.filtered_items_idx
            .iter()
            .map(|&idx| self.items.get(idx).unwrap())
            .collect()
    }

    pub fn update_item(
        &mut self,
        name: &str,
        saved: Option<bool>,
        active: Option<bool>,
    ) {
        if let Some(item) = self.items.iter_mut().find(|i| i.name == name) {
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
                new_selected
                    .min(self.filtered_items_idx.len().saturating_sub(1)),
            ));
        }
    }

    pub fn remove_item(&mut self, idx: usize, item: MenuItem) {
        self.items.retain(|i| i.name != item.name);
        self.list_state.select(Some(idx.saturating_sub(1)));
    }

    pub fn update_filter_and_reset(&mut self, input: &str) {
        self.update_filter(input);
        self.reset_position();
    }

    pub fn update_filter(&mut self, input: &str) {
        if input.is_empty() {
            self.filtered_items_idx = (0..self.items.len()).collect();
        } else {
            self.filtered_items_idx = self
                .items
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    self.matcher.fuzzy_match(&item.name, input).is_some()
                })
                .map(|(idx, _)| idx)
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

fn sort_items(items: &Vec<MenuItem>) -> Vec<MenuItem> {
    let mut items_priorities: Vec<(&MenuItem, u32)> = items
        .iter()
        .map(|item| {
            let mut priority_score = 0u32;
            if item.active {
                priority_score += 1;
            }

            (item, priority_score)
        })
        .collect();

    items_priorities
        .sort_by(|a, b| b.1.cmp(&a.1).then(a.0.name.cmp(&b.0.name)));

    items_priorities
        .iter()
        .map(|(item, _)| (*item).clone())
        .collect()
}

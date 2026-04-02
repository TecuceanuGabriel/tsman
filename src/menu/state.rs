use std::time::{Duration, Instant};

use ratatui::style::Style;
use tui_textarea::TextArea;

use crate::{
    menu::{item::MenuItem, items_state::ItemsState, ui_flags::UiFlags},
    persistence::{StorageKind, load_config},
    tmux::{layout::Layout as TmuxLayout, session::Session},
};

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
    pub last_key: Option<String>,
    pub last_key_instant: Option<Instant>,

    pub should_exit: bool,

    /// Cached preview: (item_name, is_layout_mode, width, content)
    preview_cache: Option<(String, bool, usize, String)>,
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
            ui_flags: UiFlags::new(ask_for_confirmation, show_preview),
            preview_scroll: 0,
            last_key: None,
            last_key_instant: None,
            should_exit: false,
            preview_cache: None,
        }
    }

    /// How long the last-key indicator stays visible.
    const KEY_DISPLAY_DURATION: Duration = Duration::from_millis(1500);

    /// Records the label of the last key pressed.
    pub fn set_last_key(&mut self, label: String) {
        self.last_key = Some(label);
        self.last_key_instant = Some(Instant::now());
    }

    /// Returns the key label if it's still within the display window.
    pub fn visible_last_key(&self) -> Option<&str> {
        match (&self.last_key, self.last_key_instant) {
            (Some(label), Some(instant))
                if instant.elapsed() < Self::KEY_DISPLAY_DURATION =>
            {
                Some(label)
            }
            _ => None,
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

    /// Returns the preview content for the selected item, using a cache to
    /// avoid re-loading and re-rendering on every frame.
    pub fn get_cached_preview(&mut self, width: usize) -> String {
        let is_layout = self.list_mode == ListMode::Layouts;
        let name = match self.items.get_selected_item() {
            Some((_, item)) => item.name,
            None => return String::new(),
        };

        if let Some((ref cn, ci, cw, ref content)) = self.preview_cache {
            if cn == &name && ci == is_layout && cw == width {
                return content.clone();
            }
        }

        let content = if is_layout {
            load_config(StorageKind::Layout, &name)
                .ok()
                .and_then(|yaml| serde_yaml::from_str::<TmuxLayout>(&yaml).ok())
                .map(|layout| layout.get_preview(width))
                .unwrap_or_default()
        } else {
            load_config(StorageKind::Session, &name)
                .ok()
                .and_then(|yaml| serde_yaml::from_str::<Session>(&yaml).ok())
                .map(|session| session.get_preview())
                .unwrap_or_default()
        };

        self.preview_cache = Some((name, is_layout, width, content.clone()));
        content
    }
}

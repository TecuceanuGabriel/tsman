//! Tmux layout model - [`Layout`] -> [`LayoutWindow`], capturing structure without work dirs.
use serde::{Deserialize, Serialize};

use super::session::{Session, Window};

/// A window in a layout template - captures structure but not working directories.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LayoutWindow {
    pub index: String,
    pub name: String,
    pub layout: String,
    pub pane_count: usize,
}

/// A reusable session template - window/pane structure without working directories.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Layout {
    pub name: String,
    pub windows: Vec<LayoutWindow>,
}

impl From<&Window> for LayoutWindow {
    fn from(window: &Window) -> Self {
        LayoutWindow {
            index: window.index.clone(),
            name: window.name.clone(),
            layout: window.layout.clone(),
            pane_count: window.panes.len(),
        }
    }
}

impl From<&Session> for Layout {
    fn from(session: &Session) -> Self {
        Layout {
            name: session.name.clone(),
            windows: session.windows.iter().map(LayoutWindow::from).collect(),
        }
    }
}

impl LayoutWindow {
    /// Returns a one-line preview showing the window name and pane count.
    pub fn get_preview(&self) -> String {
        let panes_label = if self.pane_count == 1 {
            "1 pane".to_string()
        } else {
            format!("{} panes", self.pane_count)
        };

        format!("{}: {}", self.name, panes_label)
    }
}

impl Layout {
    /// Returns a tree-like preview of the layout and its windows.
    pub fn get_preview(&self) -> String {
        let mut preview = format!("{}:\n", self.name);

        for (i, window) in self.windows.iter().enumerate() {
            let is_last = i == self.windows.len() - 1;
            let branch = if is_last {
                " ╚══ "
            } else {
                " ╠══ "
            };
            preview += &format!("{}{}\n", branch, window.get_preview());
        }

        preview
    }
}

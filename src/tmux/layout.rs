//! Tmux layout model
//!
//! Serializable data structures representing a layout template.
//! A layout captures the window/pane structure of a tmux session
//! without working directories, allowing reuse across projects.
//!
//! [`Layout`] -> [`LayoutWindow`] -> [`LayoutPane`]
use serde::{Deserialize, Serialize};

use super::session::{Session, Window};

/// Represents a window within a layout template.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LayoutWindow {
    /// Index of the window.
    pub index: String,
    /// Name of the window.
    pub name: String,
    /// Tmux layout string describing the window structure.
    pub layout: String,
    /// Number of panes in the window.
    pub pane_count: usize,
}

/// A layout is a session template that captures window/pane structure
/// but NOT working directories.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Layout {
    /// Name of the layout.
    pub name: String,
    /// List of windows in the layout.
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
    /// Returns a textual preview of the window.
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
    /// Returns a textual preview of the layout, including all windows.
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

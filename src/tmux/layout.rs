//! Tmux layout model
//!
//! Serializable data structures representing a layout template.
//! A layout captures the window/pane structure of a tmux session
//! without working directories, allowing reuse across projects.
//!
//! [`Layout`] -> [`LayoutWindow`] -> [`LayoutPane`]
use serde::{Deserialize, Serialize};

use super::session::{Pane, Session, Window};

/// Represents a pane within a layout template.
/// Unlike [`Pane`], this does not store a working directory.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LayoutPane {
    /// Index of the pane.
    pub index: String,
    /// Command to run in the pane, if any.
    pub current_command: Option<String>,
}

/// Represents a window within a layout template.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LayoutWindow {
    /// Index of the window.
    pub index: String,
    /// Name of the window.
    pub name: String,
    /// Tmux layout string describing the window structure.
    pub layout: String,
    /// List of panes inside the window.
    pub panes: Vec<LayoutPane>,
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

impl From<&Pane> for LayoutPane {
    fn from(pane: &Pane) -> Self {
        LayoutPane {
            index: pane.index.clone(),
            current_command: pane.current_command.clone(),
        }
    }
}

impl From<&Window> for LayoutWindow {
    fn from(window: &Window) -> Self {
        LayoutWindow {
            index: window.index.clone(),
            name: window.name.clone(),
            layout: window.layout.clone(),
            panes: window.panes.iter().map(LayoutPane::from).collect(),
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

impl LayoutPane {
    /// Returns a textual preview of the pane.
    ///
    /// # Arguments
    /// * `show_index` - Whether to include the pane index in the preview.
    pub fn get_preview(&self, show_index: bool) -> String {
        let mut preview = String::new();

        if show_index {
            preview += &format!("({}) ", self.index);
        }

        preview += match self.current_command.as_ref() {
            Some(cmd) => cmd,
            None => "_",
        };

        preview
    }
}

impl LayoutWindow {
    /// Returns a textual preview of the window and its panes.
    ///
    /// # Arguments
    /// * `add_connector` - Whether to draw a connector line before panes.
    pub fn get_preview(&self, add_connector: bool) -> String {
        if self.panes.len() == 1 {
            return format!(
                "{}: {}\n",
                self.name,
                self.panes[0].get_preview(false)
            );
        }

        let mut preview = format!("{}:\n", self.name);

        let connector = if add_connector { "║" } else { " " };

        let mut pane_idx = 0;
        while pane_idx < self.panes.len() - 1 {
            preview += &format!(
                " {}  ╠═ {}\n",
                connector,
                self.panes[pane_idx].get_preview(true)
            );
            pane_idx += 1;
        }

        preview += &format!(
            " {}  ╚═ {}\n",
            connector,
            self.panes[pane_idx].get_preview(true)
        );

        preview
    }
}

impl Layout {
    /// Returns a textual preview of the layout, including all windows and panes.
    pub fn get_preview(&self) -> String {
        let mut preview = format!("{}:\n", self.name);

        let mut window_idx = 0;
        while window_idx < self.windows.len() - 1 {
            let window = &self.windows[window_idx];
            let end_connector =
                if window.panes.len() > 1 { "╦═" } else { "" };

            preview +=
                &format!(" ╠══{} {}", end_connector, window.get_preview(true));
            window_idx += 1;
        }

        let last_window = &self.windows[window_idx];
        let end_connector = if last_window.panes.len() > 1 {
            "╦═"
        } else {
            ""
        };

        preview += &format!(
            " ╚══{} {}",
            end_connector,
            last_window.get_preview(false)
        );

        preview
    }
}

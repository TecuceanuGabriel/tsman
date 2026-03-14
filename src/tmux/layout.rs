//! Tmux layout model - [`Layout`] -> [`LayoutWindow`], capturing structure without work dirs.
use serde::{Deserialize, Serialize};

use super::layout_parser;
use super::layout_renderer;
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

const WINDOW_BOX_HEIGHT: usize = 6;

impl LayoutWindow {
    /// Returns a visual box-drawing preview of the window's pane layout.
    ///
    /// Falls back to a text description if parsing fails or space is too small.
    pub fn get_preview(&self, width: usize, height: usize) -> Vec<String> {
        if let Ok(node) = layout_parser::parse(&self.layout)
            && let Some(lines) =
                layout_renderer::render(&node, &self.name, width, height)
        {
            return lines;
        }
        // Fallback: simple text
        let panes_label = if self.pane_count == 1 {
            "1 pane".to_string()
        } else {
            format!("{} panes", self.pane_count)
        };
        vec![format!("{}: {}", self.name, panes_label)]
    }
}

impl Layout {
    /// Returns a visual preview of all windows in the layout.
    pub fn get_preview(&self, width: usize) -> String {
        let mut lines: Vec<String> = Vec::new();

        for (i, window) in self.windows.iter().enumerate() {
            if i > 0 {
                lines.push(String::new());
            }
            lines.extend(window.get_preview(width, WINDOW_BOX_HEIGHT));
        }

        lines.join("\n")
    }
}

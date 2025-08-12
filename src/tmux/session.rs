use serde::{Deserialize, Serialize};

/// Represents a tmux pane that lives inside a tmux window.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Pane {
    /// Index of the pane.
    pub index: String,
    /// Command running in the pane currently, if any.
    pub current_command: Option<String>,
    /// Working directory of the pane.
    pub work_dir: String,
}

/// Represents a tmux window that has one or more panes.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Window {
    /// Index of the window.
    pub index: String,
    /// Name of the window.
    pub name: String,
    /// Tmux layout string describing the window structure.
    pub layout: String,
    /// List of panes inside the window.
    pub panes: Vec<Pane>,
}

/// Represents a tmux session that has one or more windows.
#[derive(Serialize, Deserialize, Debug)]
pub struct Session {
    /// Name of the session.
    pub name: String,
    /// Default working directory for new panes.
    pub work_dir: String,
    /// List of windows inside the session.
    pub windows: Vec<Window>,
}

impl Pane {
    /// Returns a textual preview of the pane.
    ///
    /// # Arguments
    ///
    /// * `show_index` - Whether to include the pane index in the preview.
    ///
    /// # Example
    ///
    /// ```
    /// let pane = Pane {
    ///     index: "0".into(),
    ///     current_command: Some("bash".into()),
    ///     work_dir: "...".into()
    /// };
    /// assert_eq!(pane.get_preview(true), "(0) bash");
    /// ```
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

impl Window {
    /// Returns a textual preview of the window and its panes.
    ///
    /// # Arguments
    ///
    /// * `add_connector` - Whether to draw a connector line before panes.
    ///
    /// This method formats the window name, followed by each pane preview,
    /// in a tree-like visual layout.
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

impl Session {
    /// Returns a textual preview of the session, including all windows and panes.
    ///
    /// This method creates a tree-like view of the tmux session, showing the
    /// hierarchy from session → windows → panes.
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
            last_window.get_preview(false) // no need to add connector on last window
        );

        preview
    }
}

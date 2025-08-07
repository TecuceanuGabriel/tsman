use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Pane {
    pub index: String,
    pub current_command: Option<String>,
    pub work_dir: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Window {
    pub index: String,
    pub name: String,
    pub layout: String,
    pub panes: Vec<Pane>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Session {
    pub name: String,
    pub work_dir: String,
    pub windows: Vec<Window>,
}

impl Pane {
    pub fn get_preview(&self, show_index: bool) -> String {
        let mut preview = String::new();

        if show_index {
            preview += &format!("({}) ", self.index);
        }

        preview += &format!(
            "{}",
            match self.current_command.as_ref() {
                Some(cmd) => cmd,
                None => "_",
            }
        );

        preview
    }
}

impl Window {
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

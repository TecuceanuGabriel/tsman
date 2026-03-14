use std::fmt;

/// A session or layout entry displayed in the menu.
#[derive(Debug, Clone)]
pub struct MenuItem {
    pub name: String,
    /// Whether this item has a config saved to disk.
    pub saved: bool,
    /// Whether this item corresponds to a currently running tmux session.
    pub active: bool,
}

impl MenuItem {
    /// Creates a new menu item.
    pub fn new(name: String, saved: bool, active: bool) -> Self {
        Self {
            name,
            saved,
            active,
        }
    }
}

impl fmt::Display for MenuItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let saved_indicator = if !self.saved { "* " } else { "" };
        let active_indicator = if self.active { " (active)" } else { "" };

        write!(f, "{}{}{}", saved_indicator, self.name, active_indicator)
    }
}

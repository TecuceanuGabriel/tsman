use std::fmt;

/// A single item in the menu list.
#[derive(Debug, Clone)]
pub struct MenuItem {
    /// The session name.
    pub name: String,
    /// Whether this session is saved to disk.
    pub saved: bool,
    /// Whether this session is currently active.
    pub active: bool,
}

impl MenuItem {
    /// Creates a new menu item.
    ///
    /// # Arguments
    /// * `name` - The session name.
    /// * `saved` - Whether the session is saved to storage.
    /// * `active` - Whether the session is currently active.
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

/// Toggleable UI settings passed via CLI flags.
pub struct UiFlags {
    pub ask_for_confirmation: bool,
    pub show_preview: bool,
}

impl UiFlags {
    /// Creates flags from CLI arguments.
    pub fn new(ask_for_confirmation: bool, show_preview: bool) -> Self {
        Self {
            ask_for_confirmation,
            show_preview,
        }
    }
}

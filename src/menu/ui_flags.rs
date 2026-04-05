/// Toggleable UI settings derived from config.
pub struct UiFlags {
    pub ask_for_confirmation: bool,
    pub show_preview: bool,
    pub show_key_presses: bool,
}

impl UiFlags {
    pub fn new(
        ask_for_confirmation: bool,
        show_preview: bool,
        show_key_presses: bool,
    ) -> Self {
        Self {
            ask_for_confirmation,
            show_preview,
            show_key_presses,
        }
    }
}

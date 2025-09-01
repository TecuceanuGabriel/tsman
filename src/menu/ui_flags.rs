pub struct UiFlags {
    pub ask_for_confirmation: bool,
    pub show_confirmation_popup: bool,
    pub show_preview: bool,
    pub show_help: bool,
}

impl UiFlags {
    pub fn new(ask_for_confirmation: bool, show_preview: bool) -> Self {
        Self {
            ask_for_confirmation,
            show_confirmation_popup: false,
            show_preview,
            show_help: false,
        }
    }
}

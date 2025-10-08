pub struct UiFlags {
    pub ask_for_confirmation: bool,
    pub show_preview: bool,
}

impl UiFlags {
    pub fn new(ask_for_confirmation: bool, show_preview: bool) -> Self {
        Self {
            ask_for_confirmation,
            show_preview,
        }
    }
}

use ratatui::{DefaultTerminal, Frame, widgets::ListState};

use anyhow::{Context, Result};

struct MenuUi {
    all_items: Vec<String>,
    filtered_items: Vec<String>,
    input: String,
    list_state: ListState,
    exit: bool,
}

impl MenuUi {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        todo!()
    }

    fn draw(&self, frame: &mut Frame) {
        todo!()
    }

    fn handle_events(&mut self) -> Result<()> {
        todo!()
    }

    fn new(items: Vec<String>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            all_items: items.clone(),
            filtered_items: items,
            input: String::new(),
            list_state,
            exit: false,
        }
    }
}

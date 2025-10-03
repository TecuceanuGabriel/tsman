use std::rc::Rc;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Flex, Layout, Margin, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, Paragraph},
};

use crate::{
    menu::{
        items_state::ItemsState,
        state::{MenuMode, MenuState},
    },
    persistence::load_session_from_config,
    tmux::session::Session,
};

const HIGHLIGHT_STYLE: Style = Style::new().bg(Color::Blue);
const SUBTLE_STYLE: Style = Style::new().fg(Color::DarkGray);
const POPUP_STYLE: Style = Style::new().fg(Color::Blue).bg(Color::Gray);
const PROMPT_STYLE: Style = Style::new().fg(Color::Green);

const PREVIEW_WIDTH_RATIO: u16 = 40;

const CONFIRMATION_POPUP_WIDTH: u16 = 15;
const CONFIRMATION_POPUP_HEIGHT: u16 = 3;

const HELP_POPUP_WIDTH: u16 = 60;
const HELP_POPUP_HEIGHT: u16 = 14;

pub trait MenuRenderer {
    fn draw(&self, frame: &mut Frame, state: &mut MenuState);
}

pub struct DefaultMenuRenderer;

impl MenuRenderer for DefaultMenuRenderer {
    fn draw(&self, frame: &mut Frame, state: &mut MenuState) {
        let chunks = crate_main_layout(frame.area());
        let content_chunks =
            create_content_layout(chunks[0], state.ui_flags.show_preview);

        let left_content_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(3)])
            .split(content_chunks[0]);

        render_results_list(frame, left_content_chunks[0], &mut state.items);

        render_input_field(frame, left_content_chunks[1], state);

        render_help_hint(frame, chunks[1]);

        if state.ui_flags.show_preview {
            draw_preview_pane(frame, content_chunks[1], &state.items);
        }

        if state.mode == MenuMode::ConfirmationPopup {
            draw_confirmation_popup(frame);
        }

        if state.mode == MenuMode::HelpPopup {
            draw_help_popup(frame);
        }
    }
}

fn crate_main_layout(area: Rect) -> Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(area)
}

fn create_content_layout(area: Rect, show_preview: bool) -> Rc<[Rect]> {
    let constrains = if show_preview {
        vec![
            Constraint::Percentage(100 - PREVIEW_WIDTH_RATIO),
            Constraint::Percentage(PREVIEW_WIDTH_RATIO),
        ]
    } else {
        vec![Constraint::Percentage(100)]
    };

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constrains)
        .split(area)
}

fn render_results_list(
    frame: &mut Frame,
    area: Rect,
    items_state: &mut ItemsState,
) {
    let items: Vec<String> = items_state
        .get_filtered_items()
        .iter()
        .map(|i| i.to_string())
        .collect();

    let results_block = Block::default().borders(Borders::ALL).title("Results");

    if items.is_empty() {
        frame.render_widget(
            Paragraph::new("No results...")
                .block(results_block)
                .style(SUBTLE_STYLE),
            area,
        );
        return;
    }

    let list = List::new(items)
        .block(results_block)
        .highlight_style(HIGHLIGHT_STYLE);

    frame.render_stateful_widget(list, area, &mut items_state.list_state);
}

fn render_input_field(frame: &mut Frame, area: Rect, state: &mut MenuState) {
    let input_block = Block::default().borders(Borders::ALL).title("Search");

    frame.render_widget(input_block, area);

    let input_area = area.inner(Margin {
        horizontal: 1,
        vertical: 1,
    });

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(2), Constraint::Min(1)].as_ref())
        .split(input_area);

    let prompt = Paragraph::new("> ").style(PROMPT_STYLE);
    frame.render_widget(prompt, chunks[0]);

    frame.render_widget(&state.input, chunks[1]);
}

fn render_help_hint(frame: &mut Frame, area: Rect) {
    let help_hint = Paragraph::new("C-h: Help | Esc: Quit")
        .alignment(Alignment::Center)
        .style(SUBTLE_STYLE);

    frame.render_widget(help_hint, area);
}

fn draw_preview_pane(frame: &mut Frame, chunk: Rect, items: &ItemsState) {
    let preview_block = Block::default().borders(Borders::ALL).title("Preview");

    let preview_content = generate_preview_content(items);
    let preview = Paragraph::new(preview_content).block(preview_block);

    frame.render_widget(preview, chunk);
}

fn generate_preview_content(items: &ItemsState) -> String {
    let Some((_, selection)) = items.get_selected_item() else {
        return String::new();
    };

    load_session_from_config(&selection.name)
        .ok()
        .and_then(|session_str| {
            serde_yaml::from_str::<Session>(&session_str).ok()
        })
        .map(|session| session.get_preview())
        .unwrap_or_default()
}

fn draw_confirmation_popup(f: &mut Frame) {
    let popup_area = create_centered_rect(
        f.area(),
        CONFIRMATION_POPUP_WIDTH,
        CONFIRMATION_POPUP_HEIGHT,
    );

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title("Confirm")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .style(POPUP_STYLE);

    let paragraph = Paragraph::new(Line::from("Y/n"))
        .block(block)
        .alignment(Alignment::Center);

    f.render_widget(paragraph, popup_area);
}

fn draw_help_popup(f: &mut Frame) {
    let popup_area =
        create_centered_rect(f.area(), HELP_POPUP_WIDTH, HELP_POPUP_HEIGHT);

    f.render_widget(Clear, popup_area);

    let navigation_block = Block::default()
        .title("Navigation")
        .borders(Borders::ALL)
        .style(POPUP_STYLE);

    let session_block = Block::default()
        .title("Session Actions")
        .borders(Borders::ALL)
        .style(POPUP_STYLE);

    let ui_block = Block::default()
        .title("UI Controls")
        .borders(Borders::ALL)
        .style(POPUP_STYLE);

    let popup_block = Block::default()
        .title("Popup")
        .borders(Borders::ALL)
        .style(POPUP_STYLE);

    let navigation_text = vec![
        Line::from("Esc/C-c → Close"),
        Line::from("↑/C-p   → Previous item"),
        Line::from("↓/C-n   → Next item"),
    ];

    let session_text = vec![
        Line::from("C-e   → Edit session"),
        Line::from("C-d   → Delete/kill"),
        Line::from("C-s   → Save session"),
        Line::from("C-k   → Kill session"),
        Line::from("Enter → Open session"),
    ];

    let ui_text = vec![
        Line::from("C-t → Toggle preview"),
        Line::from("C-h → Toggle help"),
        Line::from("C-w → Delete last word"),
    ];

    let popup_text = vec![
        Line::from("y/Y/Enter → Confirm"),
        Line::from("n/N/Esc/q → Abort"),
    ];

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(HELP_POPUP_HEIGHT / 2),
            Constraint::Length(HELP_POPUP_HEIGHT / 2),
        ])
        .split(popup_area);

    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    f.render_widget(
        Paragraph::new(navigation_text).block(navigation_block),
        top_chunks[0],
    );
    f.render_widget(
        Paragraph::new(session_text).block(session_block),
        top_chunks[1],
    );
    f.render_widget(Paragraph::new(ui_text).block(ui_block), bottom_chunks[0]);
    f.render_widget(
        Paragraph::new(popup_text).block(popup_block),
        bottom_chunks[1],
    );
}

fn create_centered_rect(area: Rect, length_x: u16, length_y: u16) -> Rect {
    let vertical =
        Layout::vertical([Constraint::Length(length_y)]).flex(Flex::Center);
    let horizontal =
        Layout::horizontal([Constraint::Length(length_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

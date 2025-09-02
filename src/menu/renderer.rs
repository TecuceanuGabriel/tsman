use std::rc::Rc;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Flex, Layout, Margin, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListState, Paragraph},
};

use crate::{
    menu::{item::MenuItem, items_state::ItemsState, menu_state::MenuState},
    persistence::load_session_from_config,
    tmux::session::Session,
};

pub trait MenuRenderer {
    fn draw(&mut self, frame: &mut Frame, state: &mut MenuState);
}

pub struct DefaultMenuRenderer;

impl MenuRenderer for DefaultMenuRenderer {
    fn draw(&mut self, frame: &mut Frame, state: &mut MenuState) {
        let main_chunks = crate_main_layout(frame.area());
        let content_chunks =
            create_content_layout(main_chunks[0], state.ui_flags.show_preview);

        let left_content = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(3)])
            .split(content_chunks[0]);

        render_results_list(
            frame,
            left_content[0],
            &state.items.filtered_items,
            &mut state.items.list_state,
        );

        render_input_field(frame, left_content[1], &state.items.input);

        render_help_hint(frame, main_chunks[1]);

        if state.ui_flags.show_preview {
            draw_preview_pane(frame, content_chunks[1], &state.items);
        }

        if state.ui_flags.ask_for_confirmation
            && state.ui_flags.show_confirmation_popup
        {
            draw_confirmation_popup(frame);
        }

        if state.ui_flags.show_help {
            draw_help_popup(frame);
        }
    }
}

fn crate_main_layout(area: Rect) -> Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(1), // help hint
        ])
        .split(area)
}

fn create_content_layout(area: Rect, show_preview: bool) -> Rc<[Rect]> {
    if show_preview {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(60),
                Constraint::Percentage(40),
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(area)
    }
}

fn render_results_list(
    frame: &mut Frame,
    area: Rect,
    filtered_items: &Vec<MenuItem>,
    list_state: &mut ListState,
) {
    let items: Vec<String> =
        filtered_items.iter().map(|i| i.to_string()).collect();

    let block = Block::default().borders(Borders::ALL).title("Results");

    if items.is_empty() {
        frame.render_widget(
            Paragraph::new("No results...")
                .block(block)
                .style(Style::default().fg(Color::DarkGray)),
            area,
        );
        return;
    }

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(Color::Blue));

    frame.render_stateful_widget(list, area, list_state);
}

fn render_input_field(frame: &mut Frame, area: Rect, input: &str) {
    let input_block = Block::default().borders(Borders::ALL).title("Search");

    frame.render_widget(input_block, area);

    let input_text = Paragraph::new("> ".to_string() + input)
        .style(Style::default().fg(Color::Green));

    frame.render_widget(
        input_text,
        area.inner(Margin {
            horizontal: 1,
            vertical: 1,
        }),
    );
}

fn render_help_hint(frame: &mut Frame, area: Rect) {
    let help_hint = Paragraph::new("C-h: Help | Esc: Quit")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));

    frame.render_widget(help_hint, area);
}

fn draw_preview_pane(frame: &mut Frame, chunk: Rect, items: &ItemsState) {
    let preview_block = Block::default().borders(Borders::ALL).title("Preview");

    let preview_content = generate_preview_content(items);
    let preview = Paragraph::new(preview_content).block(preview_block);

    frame.render_widget(preview, chunk);
}

fn generate_preview_content(items: &ItemsState) -> String {
    if let Some(selection_idx) = items.list_state.selected()
        && let Some(selection) = items.filtered_items.get(selection_idx)
        && let Ok(session_str) = load_session_from_config(&selection.name)
    {
        let session: Session = serde_yaml::from_str(&session_str).ok().unwrap();
        return session.get_preview();
    }

    "".to_string()
}

fn draw_confirmation_popup(f: &mut Frame) {
    let popup_area = create_centered_rect(f.area(), 15, 3);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title("Confirm")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray));

    let paragraph = Paragraph::new(Line::from("Y/n"))
        .block(block)
        .alignment(Alignment::Center);

    f.render_widget(paragraph, popup_area);
}

fn draw_help_popup(f: &mut Frame) {
    let popup_area = create_centered_rect(f.area(), 60, 14);

    f.render_widget(Clear, popup_area);

    let navigation_block = Block::default()
        .title("Navigation")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray));

    let session_block = Block::default()
        .title("Session Actions")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray));

    let ui_block = Block::default()
        .title("UI Controls")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray));

    let popup_block = Block::default()
        .title("Popup")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray));

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
        .constraints([Constraint::Length(7), Constraint::Length(7)])
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

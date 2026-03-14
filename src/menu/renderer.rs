use std::rc::Rc;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Flex, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, Paragraph,
        Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
    },
};

use crate::{
    menu::{
        items_state::ItemsState,
        state::{ListMode, MenuMode, MenuState},
    },
    persistence::{StorageKind, load_config},
    tmux::{layout::Layout as TmuxLayout, session::Session},
};

// Monokai color palette
const MONOKAI_RED: Color = Color::Rgb(249, 38, 114);
const MONOKAI_ORANGE: Color = Color::Rgb(253, 151, 31);
const MONOKAI_GREEN: Color = Color::Rgb(166, 226, 46);
const MONOKAI_CYAN: Color = Color::Rgb(102, 217, 239);
const MONOKAI_PURPLE: Color = Color::Rgb(174, 129, 255);
const MONOKAI_COMMENT: Color = Color::Rgb(117, 113, 94);
const MONOKAI_FG: Color = Color::Rgb(248, 248, 242);

struct Theme {
    accent: Color,
    highlight: Style,
    border: Style,
    prompt: Style,
}

const SESSIONS_THEME: Theme = Theme {
    accent: MONOKAI_CYAN,
    highlight: Style::new().bg(Color::Rgb(26, 74, 90)),
    border: Style::new().fg(MONOKAI_CYAN),
    prompt: Style::new().fg(MONOKAI_CYAN),
};

const LAYOUTS_THEME: Theme = Theme {
    accent: MONOKAI_PURPLE,
    highlight: Style::new().bg(Color::Rgb(58, 42, 90)),
    border: Style::new().fg(MONOKAI_PURPLE),
    prompt: Style::new().fg(MONOKAI_PURPLE),
};

fn theme_for(list_mode: &ListMode) -> &'static Theme {
    match list_mode {
        ListMode::Sessions => &SESSIONS_THEME,
        ListMode::Layouts => &LAYOUTS_THEME,
    }
}

const SUBTLE_STYLE: Style = Style::new().fg(MONOKAI_COMMENT);
const POPUP_STYLE: Style =
    Style::new().fg(MONOKAI_CYAN).bg(Color::Rgb(39, 40, 34));
const ERROR_POPUP_STYLE: Style =
    Style::new().fg(MONOKAI_RED).bg(Color::Rgb(39, 40, 34));
const RENAME_PROMPT_STYLE: Style = Style::new().fg(MONOKAI_ORANGE);

const PREVIEW_WIDTH_RATIO: u16 = 40;

const CONFIRMATION_POPUP_WIDTH: u16 = 15;
const CONFIRMATION_POPUP_HEIGHT: u16 = 3;

const HELP_POPUP_WIDTH: u16 = 60;
const HELP_POPUP_HEIGHT: u16 = 16;

/// Draws the menu UI to a ratatui [`Frame`].
pub trait MenuRenderer {
    fn draw(&self, frame: &mut Frame, state: &mut MenuState);
}

/// Default renderer with list, filter input, preview pane, and popups.
pub struct DefaultMenuRenderer;

impl MenuRenderer for DefaultMenuRenderer {
    fn draw(&self, frame: &mut Frame, state: &mut MenuState) {
        let theme = theme_for(&state.list_mode);
        let chunks = crate_main_layout(frame.area());
        let content_chunks =
            create_content_layout(chunks[0], state.ui_flags.show_preview);

        let left_content_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(3)])
            .split(content_chunks[0]);

        render_results_list(
            frame,
            left_content_chunks[0],
            &mut state.items,
            &state.list_mode,
            theme,
        );

        render_input_field(frame, left_content_chunks[1], state, theme);

        render_help_hint(frame, chunks[1], &state.list_mode, theme);

        if state.ui_flags.show_preview {
            draw_preview_pane(
                frame,
                content_chunks[1],
                &state.items,
                &state.list_mode,
                state.preview_scroll,
                theme,
            );
        }

        match &state.mode {
            MenuMode::ConfirmationPopup => draw_confirmation_popup(frame),
            MenuMode::HelpPopup => draw_help_popup(frame),
            MenuMode::ErrorPopup(message) => draw_error(frame, message),
            _ => {}
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
    list_mode: &ListMode,
    theme: &Theme,
) {
    let results_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme.border)
        .title("Results");

    let filtered = items_state.get_filtered_items();

    if filtered.is_empty() {
        frame.render_widget(
            Paragraph::new("No results...")
                .block(results_block)
                .style(SUBTLE_STYLE),
            area,
        );
        return;
    }

    let items: Vec<ListItem> = filtered
        .iter()
        .map(|(item, match_indices)| {
            styled_list_item(item, list_mode, match_indices)
        })
        .collect();

    let item_count = filtered.len();

    let list = List::new(items)
        .block(results_block)
        .highlight_style(theme.highlight);

    frame.render_stateful_widget(list, area, &mut items_state.list_state);

    let visible_height = area.height.saturating_sub(2) as usize;
    if item_count > visible_height {
        let mut scrollbar_state = ScrollbarState::new(item_count)
            .position(items_state.list_state.selected().unwrap_or(0));
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .style(Style::new().fg(MONOKAI_COMMENT));
        frame.render_stateful_widget(
            scrollbar,
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

fn styled_list_item<'a>(
    item: &crate::menu::item::MenuItem,
    list_mode: &ListMode,
    match_indices: &[usize],
) -> ListItem<'a> {
    let mut spans = Vec::new();

    if *list_mode == ListMode::Sessions {
        if item.active && item.saved {
            spans.push(Span::styled(
                "\u{25cf} ",
                Style::new().fg(MONOKAI_GREEN),
            ));
        } else if item.active {
            spans.push(Span::styled(
                "\u{25cf} ",
                Style::new().fg(MONOKAI_ORANGE),
            ));
        } else {
            spans.push(Span::raw("  "));
        }
    }

    let is_inactive = *list_mode == ListMode::Sessions && !item.active;
    let default_style = if is_inactive {
        SUBTLE_STYLE
    } else {
        Style::default()
    };

    if match_indices.is_empty() {
        spans.push(Span::styled(item.name.clone(), default_style));
    } else {
        let match_style =
            Style::new().fg(MONOKAI_RED).add_modifier(Modifier::BOLD);
        for (i, ch) in item.name.chars().enumerate() {
            let s = ch.to_string();
            if match_indices.contains(&i) {
                spans.push(Span::styled(s, match_style));
            } else {
                spans.push(Span::styled(s, default_style));
            }
        }
    }

    ListItem::new(Line::from(spans))
}

fn render_input_field(
    frame: &mut Frame,
    area: Rect,
    state: &mut MenuState,
    theme: &Theme,
) {
    let title;
    let prompt_style;
    let input;

    match state.mode {
        MenuMode::Rename => {
            title = "Rename";
            prompt_style = RENAME_PROMPT_STYLE;
            input = &state.rename_input;
        }
        MenuMode::CreateFromLayoutName => {
            title = "Session name";
            prompt_style = RENAME_PROMPT_STYLE;
            input = &state.rename_input;
        }
        MenuMode::CreateFromLayoutWorkdir => {
            title = "Working directory";
            prompt_style = RENAME_PROMPT_STYLE;
            input = &state.rename_input;
        }
        _ => {
            title = "Search";
            prompt_style = theme.prompt;
            input = &state.filter_input;
        }
    }

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(prompt_style)
        .title(title);

    frame.render_widget(input_block, area);

    let input_area = area.inner(Margin {
        horizontal: 1,
        vertical: 1,
    });

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(2), Constraint::Min(1)].as_ref())
        .split(input_area);

    let prompt = Paragraph::new("> ").style(prompt_style);
    frame.render_widget(prompt, chunks[0]);

    frame.render_widget(input, chunks[1]);
}

fn render_help_hint(
    frame: &mut Frame,
    area: Rect,
    list_mode: &ListMode,
    theme: &Theme,
) {
    let accent_bold =
        Style::new().fg(theme.accent).add_modifier(Modifier::BOLD);
    let dim = SUBTLE_STYLE;
    let key_style = Style::new().fg(MONOKAI_FG);

    let mode_label = match list_mode {
        ListMode::Sessions => "[Sessions]",
        ListMode::Layouts => "[Layouts]",
    };
    let toggle_target = match list_mode {
        ListMode::Sessions => "Layouts",
        ListMode::Layouts => "Sessions",
    };

    let line = Line::from(vec![
        Span::styled(mode_label, accent_bold),
        Span::styled(" C-l", key_style),
        Span::styled(format!(": {toggle_target} | "), dim),
        Span::styled("C-h", key_style),
        Span::styled(": Help | ", dim),
        Span::styled("Esc", key_style),
        Span::styled(": Quit", dim),
    ]);

    let help_hint = Paragraph::new(line).alignment(Alignment::Center);

    frame.render_widget(help_hint, area);
}

fn draw_preview_pane(
    frame: &mut Frame,
    chunk: Rect,
    items: &ItemsState,
    list_mode: &ListMode,
    scroll: u16,
    theme: &Theme,
) {
    let preview_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme.border)
        .title("Preview");

    let available_width = chunk.width.saturating_sub(2) as usize;
    let preview_content =
        generate_preview_content(items, list_mode, available_width);
    let preview = Paragraph::new(preview_content)
        .block(preview_block)
        .scroll((scroll, 0));

    frame.render_widget(preview, chunk);
}

fn generate_preview_content(
    items: &ItemsState,
    list_mode: &ListMode,
    width: usize,
) -> String {
    let Some((_, selection)) = items.get_selected_item() else {
        return String::new();
    };

    match list_mode {
        ListMode::Sessions => {
            load_config(StorageKind::Session, &selection.name)
                .ok()
                .and_then(|yaml| serde_yaml::from_str::<Session>(&yaml).ok())
                .map(|session| session.get_preview())
                .unwrap_or_default()
        }
        ListMode::Layouts => load_config(StorageKind::Layout, &selection.name)
            .ok()
            .and_then(|yaml| serde_yaml::from_str::<TmuxLayout>(&yaml).ok())
            .map(|layout| layout.get_preview(width))
            .unwrap_or_default(),
    }
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
        Line::from("C-o   → Reload session"),
        Line::from("Enter → Open session"),
    ];

    let ui_text = vec![
        Line::from("C-t       → Toggle preview"),
        Line::from("C-h       → Toggle help"),
        Line::from("C-w       → Delete last word"),
        Line::from("S-↑ / S-↓ → Scroll preview"),
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

fn draw_error(f: &mut Frame, message: &str) {
    let popup_area = create_centered_rect(f.area(), 30, 10);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title("Error")
        .borders(Borders::ALL)
        .style(ERROR_POPUP_STYLE);

    let paragraph = Paragraph::new(message)
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph.centered(), popup_area);
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

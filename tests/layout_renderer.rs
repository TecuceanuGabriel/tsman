use tsman::tmux::{layout_parser, layout_renderer};

#[test]
fn render_single_pane() {
    let node = layout_parser::parse("1f76,80x24,0,0,0").unwrap();
    let lines = layout_renderer::render(&node, "main", 20, 5).unwrap();
    assert_eq!(lines.len(), 5);
    // Top border has name
    assert!(lines[0].contains("main"));
    // Has corners
    assert!(lines[0].starts_with("┌"));
    assert!(lines[0].ends_with("┐"));
    assert!(lines[4].starts_with("└"));
    assert!(lines[4].ends_with("┘"));
    // Interior is empty
    assert!(lines[2].starts_with("│"));
    assert!(lines[2].ends_with("│"));
}

#[test]
fn render_horizontal_split() {
    let node =
        layout_parser::parse("b1cd,190x47,0,0{95x47,0,0,1,94x47,96,0,2}")
            .unwrap();
    let lines = layout_renderer::render(&node, "editor", 21, 5).unwrap();
    // Should have a vertical divider roughly in the middle
    // Check that some interior row has a │ character not at the edges
    let mid_row = &lines[2];
    let interior_chars: Vec<char> = mid_row.chars().skip(1).collect();
    let interior_chars = &interior_chars[..interior_chars.len() - 1];
    assert!(
        interior_chars.contains(&'│'),
        "expected vertical divider in interior: {mid_row:?}"
    );
}

#[test]
fn render_vertical_split() {
    let node = layout_parser::parse("a1b2,80x24,0,0[80x12,0,0,1,80x11,0,13,2]")
        .unwrap();
    let lines = layout_renderer::render(&node, "shell", 20, 7).unwrap();
    // Should have a horizontal divider somewhere in the middle rows
    let has_hdiv = lines[1..6].iter().any(|line| {
        let chars: Vec<char> = line.chars().skip(1).collect();
        let interior = &chars[..chars.len() - 1];
        interior.contains(&'─')
    });
    assert!(has_hdiv, "expected horizontal divider: {lines:#?}");
}

#[test]
fn render_nested_splits() {
    let node = layout_parser::parse(
        "xxxx,200x50,0,0{100x50,0,0,1,99x50,101,0[99x25,101,0,2,99x24,101,26,3]}",
    )
    .unwrap();
    let lines = layout_renderer::render(&node, "dev", 30, 9).unwrap();
    // Should have both vertical and horizontal dividers
    let all_text: String = lines.join("\n");
    assert!(all_text.contains('┬'), "expected ┬ junction");
    assert!(all_text.contains('┤'), "expected ┤ junction");
}

#[test]
fn render_too_small_returns_none() {
    let node = layout_parser::parse("1f76,80x24,0,0,0").unwrap();
    assert!(layout_renderer::render(&node, "x", 2, 2).is_none());
}

#[test]
fn render_name_truncation() {
    let node = layout_parser::parse("1f76,80x24,0,0,0").unwrap();
    let lines =
        layout_renderer::render(&node, "very-long-window-name", 10, 4).unwrap();
    // Name should be truncated to fit
    assert_eq!(lines[0].chars().count(), 10);
}

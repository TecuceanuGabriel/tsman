use tsman::tmux::layout_parser::{self, LayoutBody};

#[test]
fn parse_single_pane() {
    let node = layout_parser::parse("1f76,80x24,0,0,0").unwrap();
    assert_eq!(node.width, 80);
    assert_eq!(node.height, 24);
    assert_eq!(node.body, LayoutBody::Leaf);
}

#[test]
fn parse_horizontal_split() {
    // Two panes side by side
    let node =
        layout_parser::parse("b1cd,190x47,0,0{95x47,0,0,1,94x47,96,0,2}")
            .unwrap();
    assert_eq!(node.width, 190);
    assert_eq!(node.height, 47);
    match &node.body {
        LayoutBody::HSplit { children } => {
            assert_eq!(children.len(), 2);
            assert_eq!(children[0].width, 95);
            assert_eq!(children[0].height, 47);
            assert_eq!(children[0].body, LayoutBody::Leaf);
            assert_eq!(children[1].width, 94);
            assert_eq!(children[1].height, 47);
            assert_eq!(children[1].body, LayoutBody::Leaf);
        }
        other => panic!("expected HSplit, got {other:?}"),
    }
}

#[test]
fn parse_vertical_split() {
    // Two panes stacked
    let node = layout_parser::parse("a1b2,80x24,0,0[80x12,0,0,1,80x11,0,13,2]")
        .unwrap();
    assert_eq!(node.width, 80);
    assert_eq!(node.height, 24);
    match &node.body {
        LayoutBody::VSplit { children } => {
            assert_eq!(children.len(), 2);
            assert_eq!(children[0].width, 80);
            assert_eq!(children[0].height, 12);
            assert_eq!(children[1].width, 80);
            assert_eq!(children[1].height, 11);
        }
        other => panic!("expected VSplit, got {other:?}"),
    }
}

#[test]
fn parse_nested_splits() {
    // Horizontal split where right pane is further split vertically
    let node = layout_parser::parse(
        "xxxx,200x50,0,0{100x50,0,0,1,99x50,101,0[99x25,101,0,2,99x24,101,26,3]}",
    )
    .unwrap();
    assert_eq!(node.width, 200);
    assert_eq!(node.height, 50);
    match &node.body {
        LayoutBody::HSplit { children } => {
            assert_eq!(children.len(), 2);
            assert_eq!(children[0].body, LayoutBody::Leaf);
            match &children[1].body {
                LayoutBody::VSplit { children: inner } => {
                    assert_eq!(inner.len(), 2);
                    assert_eq!(inner[0].width, 99);
                    assert_eq!(inner[0].height, 25);
                    assert_eq!(inner[1].width, 99);
                    assert_eq!(inner[1].height, 24);
                }
                other => panic!("expected nested VSplit, got {other:?}"),
            }
        }
        other => panic!("expected HSplit, got {other:?}"),
    }
}

#[test]
fn parse_three_way_horizontal() {
    let node = layout_parser::parse(
        "abcd,120x40,0,0{40x40,0,0,1,39x40,41,0,2,39x40,81,0,3}",
    )
    .unwrap();
    match &node.body {
        LayoutBody::HSplit { children } => {
            assert_eq!(children.len(), 3);
        }
        other => panic!("expected HSplit with 3 children, got {other:?}"),
    }
}

#[test]
fn parse_invalid_missing_checksum() {
    assert!(layout_parser::parse("80x24,0,0,0").is_err());
}

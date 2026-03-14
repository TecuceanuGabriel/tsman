//! Parser for tmux window layout strings.
//!
//! Tmux layout strings encode pane geometry in a compact format:
//! `<checksum>,<WxH,X,Y><body>` where body is either a leaf pane ID,
//! `{children}` for horizontal splits, or `[children]` for vertical splits.

use anyhow::{Context, Result, bail};

/// A node in the parsed layout tree.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutNode {
    pub width: u32,
    pub height: u32,
    pub body: LayoutBody,
}

/// The body of a layout node - either a leaf pane or a split container.
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutBody {
    /// A single pane (no further splits).
    Leaf,
    /// Horizontal split - panes arranged side by side. Encoded as `{...}` in tmux.
    HSplit { children: Vec<LayoutNode> },
    /// Vertical split - panes stacked top to bottom. Encoded as `[...]` in tmux.
    VSplit { children: Vec<LayoutNode> },
}

/// Parse a tmux layout string into a [`LayoutNode`] tree.
///
/// The input format is: `<4-char-hex-checksum>,<layout-tree>`
/// e.g. `"b]cd,190x47,0,0{95x47,0,0,1,94x47,96,0,2}"`
pub fn parse(layout_str: &str) -> Result<LayoutNode> {
    // Skip the checksum: find the first comma, then parse from after it
    let rest = skip_checksum(layout_str)?;
    let (node, remaining) = parse_node(rest)?;
    if !remaining.is_empty() {
        bail!("unexpected trailing content: {remaining:?}");
    }
    Ok(node)
}

/// Skip the 4-char checksum and comma prefix.
fn skip_checksum(input: &str) -> Result<&str> {
    let comma_pos = input.find(',').context("missing comma after checksum")?;
    Ok(&input[comma_pos + 1..])
}

/// Parse a single node: `WxH,X,Y` followed by body.
fn parse_node(input: &str) -> Result<(LayoutNode, &str)> {
    let (width, rest) = parse_u32_until(input, 'x').context("parsing width")?;
    let (height, rest) =
        parse_u32_until(rest, ',').context("parsing height")?;
    // Skip X position
    let (_, rest) = parse_u32_until(rest, ',').context("parsing x position")?;
    // Skip Y position
    let (_, rest) =
        parse_y_and_detect_body(rest).context("parsing y position")?;

    parse_body(rest, width, height)
}

/// Parse Y coordinate, which is followed by either a bracket, comma+pane_id, or end of input.
/// Returns (y_value, remaining_input_starting_at_body_indicator).
fn parse_y_and_detect_body(input: &str) -> Result<(u32, &str)> {
    let mut end = 0;
    for (i, c) in input.char_indices() {
        if c == '{' || c == '[' || c == ',' || c == '}' || c == ']' {
            end = i;
            break;
        }
        if !c.is_ascii_digit() {
            bail!("unexpected char {c:?} in Y coordinate");
        }
        end = i + 1;
    }
    let y: u32 = input[..end].parse().context("invalid Y coordinate")?;
    Ok((y, &input[end..]))
}

/// Parse the body part of a node (after `WxH,X,Y`).
fn parse_body(
    input: &str,
    width: u32,
    height: u32,
) -> Result<(LayoutNode, &str)> {
    let node_of = |body, rest| {
        (
            LayoutNode {
                width,
                height,
                body,
            },
            rest,
        )
    };

    if input.is_empty() {
        // End of string - this is a leaf
        return Ok(node_of(LayoutBody::Leaf, input));
    }

    match input.as_bytes()[0] {
        b'{' => {
            let (children, rest) = parse_children(&input[1..], b'}')?;
            Ok(node_of(LayoutBody::HSplit { children }, rest))
        }
        b'[' => {
            let (children, rest) = parse_children(&input[1..], b']')?;
            Ok(node_of(LayoutBody::VSplit { children }, rest))
        }
        b',' => {
            // Comma followed by pane_id - this is a leaf
            let rest = &input[1..];
            let id_end = rest
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(rest.len());
            Ok(node_of(LayoutBody::Leaf, &rest[id_end..]))
        }
        b'}' | b']' => {
            // We've reached a parent's closing bracket - treat as leaf with no pane_id
            Ok(node_of(LayoutBody::Leaf, input))
        }
        _ => bail!("unexpected char {:?} after coordinates", &input[..1]),
    }
}

/// Parse comma-separated children inside brackets until the closing `close_bracket`.
fn parse_children(
    input: &str,
    close_bracket: u8,
) -> Result<(Vec<LayoutNode>, &str)> {
    let mut children = Vec::new();
    let mut rest = input;

    loop {
        if rest.is_empty() {
            bail!("unexpected end of input, expected closing bracket");
        }
        if rest.as_bytes()[0] == close_bracket {
            rest = &rest[1..];
            break;
        }
        if !children.is_empty() {
            // Expect comma separator between children
            if rest.as_bytes()[0] != b',' {
                bail!("expected ',' between children, got {:?}", &rest[..1]);
            }
            rest = &rest[1..];
        }
        let (child, remaining) = parse_node(rest)?;
        children.push(child);
        rest = remaining;
    }

    Ok((children, rest))
}

/// Parse digits as u32 until the given delimiter, consuming the delimiter.
fn parse_u32_until(input: &str, delim: char) -> Result<(u32, &str)> {
    let pos = input
        .find(delim)
        .with_context(|| format!("expected {delim:?} delimiter"))?;
    let value: u32 = input[..pos]
        .parse()
        .with_context(|| format!("invalid number: {:?}", &input[..pos]))?;
    Ok((value, &input[pos + 1..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_pane() {
        let node = parse("1f76,80x24,0,0,0").unwrap();
        assert_eq!(node.width, 80);
        assert_eq!(node.height, 24);
        assert_eq!(node.body, LayoutBody::Leaf);
    }

    #[test]
    fn parse_horizontal_split() {
        // Two panes side by side
        let node = parse("b1cd,190x47,0,0{95x47,0,0,1,94x47,96,0,2}").unwrap();
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
        let node = parse("a1b2,80x24,0,0[80x12,0,0,1,80x11,0,13,2]").unwrap();
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
        let node = parse(
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
        let node =
            parse("abcd,120x40,0,0{40x40,0,0,1,39x40,41,0,2,39x40,81,0,3}")
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
        assert!(parse("80x24,0,0,0").is_err());
    }
}

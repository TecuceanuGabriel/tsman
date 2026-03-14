//! Renders a parsed [`LayoutNode`] tree as a box-drawing diagram.
//!
//! Produces a 2D grid of characters showing pane arrangements with
//! proportional sizing and Unicode box-drawing borders.

use super::layout_parser::{LayoutBody, LayoutNode};

const MIN_PANE_WIDTH: usize = 3;
const MIN_PANE_HEIGHT: usize = 3;

/// Render a layout node tree into lines of box-drawing characters.
///
/// `name` is overlaid into the top border. The diagram is sized to
/// fit within `width` columns and `height` rows.
/// Returns `None` if the available space is too small.
pub fn render(
    node: &LayoutNode,
    name: &str,
    width: usize,
    height: usize,
) -> Option<Vec<String>> {
    if width < MIN_PANE_WIDTH || height < MIN_PANE_HEIGHT {
        return None;
    }

    let mut grid = Grid::new(width, height);
    grid.draw_box(0, 0, width, height);
    draw_splits(&mut grid, node, 0, 0, width, height);
    grid.overlay_name(name);
    Some(grid.to_lines())
}

/// A 2D character grid for drawing box diagrams.
struct Grid {
    cells: Vec<Vec<char>>,
    width: usize,
    height: usize,
}

impl Grid {
    fn new(width: usize, height: usize) -> Self {
        Self {
            cells: vec![vec![' '; width]; height],
            width,
            height,
        }
    }

    fn set(&mut self, x: usize, y: usize, c: char) {
        if x < self.width && y < self.height {
            self.cells[y][x] = c;
        }
    }

    fn get(&self, x: usize, y: usize) -> char {
        if x < self.width && y < self.height {
            self.cells[y][x]
        } else {
            ' '
        }
    }

    /// Draw a rectangular border.
    fn draw_box(&mut self, x: usize, y: usize, w: usize, h: usize) {
        if w < 2 || h < 2 {
            return;
        }
        let right = x + w - 1;
        let bottom = y + h - 1;

        // Corners
        self.set(x, y, '┌');
        self.set(right, y, '┐');
        self.set(x, bottom, '└');
        self.set(right, bottom, '┘');

        // Horizontal edges
        for col in (x + 1)..right {
            self.set(col, y, '─');
            self.set(col, bottom, '─');
        }

        // Vertical edges
        for row in (y + 1)..bottom {
            self.set(x, row, '│');
            self.set(right, row, '│');
        }
    }

    /// Draw a vertical divider line at column `x` from `y_top` to `y_bottom` (inclusive).
    fn draw_vertical_divider(
        &mut self,
        x: usize,
        y_top: usize,
        y_bottom: usize,
    ) {
        for row in (y_top + 1)..y_bottom {
            self.set(x, row, '│');
        }
        // Junction characters at top and bottom
        self.set(x, y_top, resolve_junction(self.get(x, y_top), '┬'));
        self.set(x, y_bottom, resolve_junction(self.get(x, y_bottom), '┴'));
    }

    /// Draw a horizontal divider line at row `y` from `x_left` to `x_right` (inclusive).
    fn draw_horizontal_divider(
        &mut self,
        y: usize,
        x_left: usize,
        x_right: usize,
    ) {
        for col in (x_left + 1)..x_right {
            self.set(col, y, resolve_junction(self.get(col, y), '─'));
        }
        // Junction characters at left and right
        self.set(x_left, y, resolve_junction(self.get(x_left, y), '├'));
        self.set(x_right, y, resolve_junction(self.get(x_right, y), '┤'));
    }

    /// Overlay the window name into the top border: `┌─ name ───┐`
    fn overlay_name(&mut self, name: &str) {
        if self.width < 5 {
            return;
        }
        let max_name_len = self.width.saturating_sub(4);
        let display_name = if name.len() > max_name_len {
            &name[..max_name_len]
        } else {
            name
        };
        // Write " name " starting at column 1
        self.set(1, 0, ' ');
        for (i, c) in display_name.chars().enumerate() {
            self.set(2 + i, 0, c);
        }
        self.set(2 + display_name.len(), 0, ' ');
    }

    fn to_lines(&self) -> Vec<String> {
        self.cells.iter().map(|row| row.iter().collect()).collect()
    }
}

/// Resolve the correct junction character when a new line meets an existing border character.
fn resolve_junction(existing: char, incoming: char) -> char {
    match (existing, incoming) {
        // Vertical line meeting horizontal
        ('│', '─') | ('─', '│') => '┼',
        // Top border meeting vertical divider going down
        ('─', '┬') | ('┬', '─') | ('┬', '┬') => '┬',
        // Bottom border meeting vertical divider going up
        ('─', '┴') | ('┴', '─') | ('┴', '┴') => '┴',
        // Left border meeting horizontal divider going right
        ('│', '├') | ('├', '│') | ('├', '├') => '├',
        // Right border meeting horizontal divider going left
        ('│', '┤') | ('┤', '│') | ('┤', '┤') => '┤',
        // Cross junctions
        ('┬', '┴') | ('┴', '┬') => '┼',
        ('├', '┤') | ('┤', '├') => '┼',
        ('┬', '├') | ('├', '┬') => '┼',
        ('┬', '┤') | ('┤', '┬') => '┼',
        ('┴', '├') | ('├', '┴') => '┼',
        ('┴', '┤') | ('┤', '┴') => '┼',
        ('┼', _) | (_, '┼') => '┼',
        // Corner meeting divider
        ('┌', '┬') | ('┌', '├') => '┌',
        ('┐', '┬') | ('┐', '┤') => '┐',
        ('└', '┴') | ('└', '├') => '└',
        ('┘', '┴') | ('┘', '┤') => '┘',
        // Default: incoming wins
        (_, new) => new,
    }
}

/// Recursively draw internal split dividers within the given bounds.
fn draw_splits(
    grid: &mut Grid,
    node: &LayoutNode,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
) {
    match &node.body {
        LayoutBody::Leaf => {}
        LayoutBody::HSplit { children } => {
            let positions = distribute(w, node.width, children, |c| c.width);
            let mut cx = x;
            for (i, (child, cw)) in children.iter().zip(&positions).enumerate()
            {
                if i > 0 {
                    grid.draw_vertical_divider(cx, y, y + h - 1);
                }
                draw_splits(grid, child, cx, y, *cw, h);
                cx += cw;
            }
        }
        LayoutBody::VSplit { children } => {
            let positions = distribute(h, node.height, children, |c| c.height);
            let mut cy = y;
            for (i, (child, ch)) in children.iter().zip(&positions).enumerate()
            {
                if i > 0 {
                    grid.draw_horizontal_divider(cy, x, x + w - 1);
                }
                draw_splits(grid, child, x, cy, w, *ch);
                cy += ch;
            }
        }
    }
}

/// Distribute `total_cells` among children proportionally based on their tmux dimensions.
///
/// Each child gets at least `MIN_PANE_WIDTH` or `MIN_PANE_HEIGHT` cells (depending on axis).
/// The last child absorbs any rounding remainder.
fn distribute<F>(
    total_cells: usize,
    total_tmux: u32,
    children: &[LayoutNode],
    dimension: F,
) -> Vec<usize>
where
    F: Fn(&LayoutNode) -> u32,
{
    if children.is_empty() || total_tmux == 0 {
        return vec![];
    }

    let min_size = MIN_PANE_WIDTH.min(MIN_PANE_HEIGHT);
    let n = children.len();

    // Calculate proportional sizes
    let mut sizes: Vec<usize> = children
        .iter()
        .map(|c| {
            let proportion = dimension(c) as f64 / total_tmux as f64;
            (proportion * total_cells as f64).round() as usize
        })
        .collect();

    // Enforce minimum sizes
    for size in &mut sizes {
        if *size < min_size {
            *size = min_size;
        }
    }

    // Adjust last child to absorb rounding errors
    let used: usize = sizes[..n - 1].iter().sum();
    if total_cells > used {
        sizes[n - 1] = total_cells - used;
    }

    sizes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tmux::layout_parser;

    #[test]
    fn render_single_pane() {
        let node = layout_parser::parse("1f76,80x24,0,0,0").unwrap();
        let lines = render(&node, "main", 20, 5).unwrap();
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
        let lines = render(&node, "editor", 21, 5).unwrap();
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
        let node =
            layout_parser::parse("a1b2,80x24,0,0[80x12,0,0,1,80x11,0,13,2]")
                .unwrap();
        let lines = render(&node, "shell", 20, 7).unwrap();
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
        let lines = render(&node, "dev", 30, 9).unwrap();
        // Should have both vertical and horizontal dividers
        let all_text: String = lines.join("\n");
        assert!(all_text.contains('┬'), "expected ┬ junction");
        assert!(all_text.contains('┤'), "expected ┤ junction");
    }

    #[test]
    fn render_too_small_returns_none() {
        let node = layout_parser::parse("1f76,80x24,0,0,0").unwrap();
        assert!(render(&node, "x", 2, 2).is_none());
    }

    #[test]
    fn render_name_truncation() {
        let node = layout_parser::parse("1f76,80x24,0,0,0").unwrap();
        let lines = render(&node, "very-long-window-name", 10, 4).unwrap();
        // Name should be truncated to fit
        assert_eq!(lines[0].chars().count(), 10);
    }
}

//! Small, deterministic layout calculations. No timers, animation state, or heap work
//! beyond the returned rectangle list.

use smithay::utils::{Logical, Point, Rectangle, Size};

/// Compute a master/stack layout for `count` windows inside `area`.
///
/// The first window gets 60% of the width. Remaining windows share the right
/// side in equal-height rows. A single window fills the whole area.
pub fn tile_rects(area: Rectangle<i32, Logical>, count: usize) -> Vec<Rectangle<i32, Logical>> {
    if count == 0 {
        return Vec::new();
    }
    if count == 1 {
        return vec![area];
    }

    let width = area.size.w.max(2);
    let height = area.size.h.max(1);
    let master_width = ((width as f64 * 0.60).round() as i32).clamp(1, width - 1);
    let stack_width = width - master_width;
    let stack_count = count - 1;
    let base_height = height / stack_count as i32;
    let mut rects = Vec::with_capacity(count);

    rects.push(Rectangle::new(
        Point::from((area.loc.x, area.loc.y)),
        Size::from((master_width, height)),
    ));

    let mut y = area.loc.y;
    for index in 0..stack_count {
        let remaining_rows = (stack_count - index) as i32;
        let row_height = if index + 1 == stack_count {
            area.loc.y + height - y
        } else {
            base_height
        }.max(1);
        rects.push(Rectangle::new(
            Point::from((area.loc.x + master_width, y)),
            Size::from((stack_width, row_height)),
        ));
        y += row_height;
        if remaining_rows == 0 {
            break;
        }
    }

    rects
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> Rectangle<i32, Logical> {
        Rectangle::new(Point::from((10, 20)), Size::from((1000, 800)))
    }

    #[test]
    fn no_windows_produces_no_rectangles() {
        assert!(tile_rects(area(), 0).is_empty());
    }

    #[test]
    fn one_window_fills_area() {
        assert_eq!(tile_rects(area(), 1), vec![area()]);
    }

    #[test]
    fn two_windows_split_width_and_cover_area() {
        let rects = tile_rects(area(), 2);
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].loc, Point::from((10, 20)));
        assert_eq!(rects[0].size, Size::from((600, 800)));
        assert_eq!(rects[1].loc, Point::from((610, 20)));
        assert_eq!(rects[1].size, Size::from((400, 800)));
    }

    #[test]
    fn stack_rows_cover_full_height() {
        let rects = tile_rects(area(), 4);
        assert_eq!(rects.len(), 4);
        assert_eq!(rects[1].loc.y, 20);
        assert_eq!(rects[2].loc.y, 20 + rects[1].size.h);
        assert_eq!(rects[3].loc.y + rects[3].size.h, 820);
        assert!(rects.iter().all(|r| r.size.w > 0 && r.size.h > 0));
    }
}

use image::{ImageBuffer, ImageFormat, Rgba};
use tauri::image::Image;

const IDLE_ICON_BYTES: &[u8] = include_bytes!("../../../assets/tray/idle.png");
const ICON_SIZE: i32 = 20;
const INDICATOR_COLUMNS: usize = 7;
const INDICATOR_ROWS: usize = 3;
const INDICATOR_STATES_PER_ROW: usize = INDICATOR_COLUMNS + 1;
const INDICATOR_Y: i32 = 12;
const INDICATOR_CELL_WIDTH: i32 = 2;
const INDICATOR_CELL_HEIGHT: i32 = 2;
const INDICATOR_GAP_X: i32 = 1;
const INDICATOR_GAP_Y: i32 = 1;
const DIGIT_Y: i32 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IconState {
    Idle,
    Collecting,
    Stable,
}

#[derive(Clone, Copy, Debug)]
pub struct TrayIconRender {
    pub bpm: Option<u16>,
    pub state: IconState,
    pub tap_count: usize,
    pub stable_tap_dots: usize,
    pub theme: IconTheme,
    pub metrics: IconMetrics,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct IconMetrics;

#[derive(Clone, Copy, Debug)]
pub struct IconTheme {
    pub idle: IconColors,
    pub collecting: IconColors,
    pub stable: IconColors,
}

#[derive(Clone, Copy, Debug)]
pub struct IconColors {
    pub background: Rgba<u8>,
    pub foreground: Rgba<u8>,
    pub empty_dot: Rgba<u8>,
    pub filled_dot: Rgba<u8>,
    pub newest_dot: Rgba<u8>,
}

impl Default for IconTheme {
    fn default() -> Self {
        Self {
            idle: IconColors {
                background: Rgba([51, 65, 85, 255]),
                foreground: Rgba([248, 250, 252, 255]),
                empty_dot: Rgba([51, 65, 85, 255]),
                filled_dot: Rgba([148, 163, 184, 255]),
                newest_dot: Rgba([248, 250, 252, 255]),
            },
            collecting: IconColors {
                background: Rgba([67, 67, 67, 255]),
                foreground: Rgba([255, 255, 0, 255]),
                empty_dot: Rgba([67, 67, 67, 255]),
                filled_dot: Rgba([64, 212, 255, 255]),
                newest_dot: Rgba([64, 212, 255, 255]),
            },
            stable: IconColors {
                background: Rgba([67, 67, 67, 255]),
                foreground: Rgba([255, 255, 0, 255]),
                empty_dot: Rgba([67, 67, 67, 255]),
                filled_dot: Rgba([64, 212, 255, 255]),
                newest_dot: Rgba([64, 212, 255, 255]),
            },
        }
    }
}

#[derive(Clone, Copy)]
struct DigitStyle {
    column_widths: [i32; DIGIT_COLUMNS],
    row_heights: [i32; DIGIT_ROWS],
    y: i32,
    gap: i32,
}

const DIGIT_COLUMNS: usize = 5;
const DIGIT_ROWS: usize = 7;
const DIGIT_ROW_HEIGHTS: [i32; DIGIT_ROWS] = [2, 1, 2, 1, 2, 1, 2];
const DIGIT_BITMAPS: [[&str; 7]; 10] = [
    [
        "11111", "11011", "11011", "11011", "11011", "11011", "11111",
    ],
    [
        "00110", "01110", "00110", "00110", "00110", "00110", "11111",
    ],
    [
        "11111", "00011", "00011", "11111", "11000", "11000", "11111",
    ],
    [
        "11111", "00011", "00011", "11111", "00011", "00011", "11111",
    ],
    [
        "11011", "11011", "11011", "11111", "00011", "00011", "00011",
    ],
    [
        "11111", "11000", "11000", "11111", "00011", "00011", "11111",
    ],
    [
        "11111", "11000", "11000", "11111", "11011", "11011", "11111",
    ],
    [
        "11111", "00011", "00011", "00110", "01100", "01100", "01100",
    ],
    [
        "11111", "11011", "11011", "11111", "11011", "11011", "11111",
    ],
    [
        "11111", "11011", "11011", "11111", "00011", "00011", "11111",
    ],
];

pub fn render_tray_icon(render: TrayIconRender) -> image::ImageResult<Image<'static>> {
    let canvas = if render.state == IconState::Idle {
        render_idle_icon_pixels()?
    } else {
        render_icon_pixels(render)
    };
    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(canvas)
        .write_to(&mut std::io::Cursor::new(&mut png), ImageFormat::Png)?;
    Image::from_bytes(&png).map_err(|error| {
        image::ImageError::IoError(std::io::Error::new(std::io::ErrorKind::Other, error))
    })
}

fn render_idle_icon_pixels() -> image::ImageResult<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    Ok(image::load_from_memory(IDLE_ICON_BYTES)?.to_rgba8())
}

fn render_icon_pixels(render: TrayIconRender) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let colors = render.theme.colors_for(render.state);
    let metrics = render.metrics;
    let size = ICON_SIZE as u32;
    let mut canvas = ImageBuffer::from_pixel(size, size, colors.background);
    draw_tap_grid(
        &mut canvas,
        metrics,
        render.tap_count,
        render.stable_tap_dots,
        colors.empty_dot,
        colors.filled_dot,
        colors.newest_dot,
    );

    match render.bpm {
        Some(value) => {
            draw_centered_number(&mut canvas, metrics, value.min(999), colors.foreground)
        }
        None => draw_tap_mark(&mut canvas, metrics, colors.foreground),
    }

    canvas
}

impl IconTheme {
    fn colors_for(self, state: IconState) -> IconColors {
        match state {
            IconState::Idle => self.idle,
            IconState::Collecting => self.collecting,
            IconState::Stable => self.stable,
        }
    }
}

fn draw_tap_grid(
    canvas: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    metrics: IconMetrics,
    tap_count: usize,
    stable_tap_dots: usize,
    empty_color: Rgba<u8>,
    filled_color: Rgba<u8>,
    newest_color: Rgba<u8>,
) {
    if stable_tap_dots == 0 {
        return;
    }

    let visible_start = visible_start_state(tap_count);
    let newest_index = tap_count.checked_sub(1);
    let highlight_newest = tap_count >= stable_tap_dots;

    for row in 0..INDICATOR_ROWS {
        let row_start = row * INDICATOR_STATES_PER_ROW;

        for column in 0..INDICATOR_COLUMNS {
            let state_index = visible_start + row_start + column;
            let color = if highlight_newest && newest_index == Some(state_index) {
                newest_color
            } else if state_index < tap_count {
                filled_color
            } else {
                empty_color
            };

            draw_indicator_cell(canvas, metrics, row, column, color);
        }

        let connector_index = visible_start + row_start + INDICATOR_COLUMNS;
        if connector_index < tap_count {
            let color = if highlight_newest && newest_index == Some(connector_index) {
                newest_color
            } else {
                filled_color
            };

            draw_indicator_connector(canvas, metrics, row, color);
        }
    }
}

fn visible_start_state(tap_count: usize) -> usize {
    if tap_count == 0 {
        return 0;
    }

    let completed_groups = tap_count / INDICATOR_STATES_PER_ROW;
    let visible_group_start = completed_groups.saturating_sub(INDICATOR_ROWS - 1);

    visible_group_start * INDICATOR_STATES_PER_ROW
}

fn draw_indicator_cell(
    canvas: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    metrics: IconMetrics,
    row: usize,
    column: usize,
    color: Rgba<u8>,
) {
    let x = indicator_cell_x(column);
    let y = indicator_cell_y(row);

    draw_rect(
        canvas,
        metrics,
        x,
        y,
        INDICATOR_CELL_WIDTH,
        INDICATOR_CELL_HEIGHT,
        color,
    );
}

fn draw_indicator_connector(
    canvas: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    metrics: IconMetrics,
    row: usize,
    color: Rgba<u8>,
) {
    let y = indicator_cell_y(row);

    for gap in 0..(INDICATOR_COLUMNS - 1) {
        let x = indicator_cell_x(gap) + INDICATOR_CELL_WIDTH;
        draw_rect(
            canvas,
            metrics,
            x,
            y,
            INDICATOR_GAP_X,
            INDICATOR_CELL_HEIGHT,
            color,
        );
    }
}

fn indicator_cell_x(column: usize) -> i32 {
    column as i32 * (INDICATOR_CELL_WIDTH + INDICATOR_GAP_X)
}

fn indicator_cell_y(row: usize) -> i32 {
    INDICATOR_Y + row as i32 * (INDICATOR_CELL_HEIGHT + INDICATOR_GAP_Y)
}

fn draw_tap_mark(
    canvas: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    metrics: IconMetrics,
    color: Rgba<u8>,
) {
    draw_rect(canvas, metrics, 6, 2, 8, 2, color);
    draw_rect(canvas, metrics, 9, 2, 2, 8, color);
    draw_rect(canvas, metrics, 7, 8, 6, 2, color);
}

fn draw_centered_number(
    canvas: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    metrics: IconMetrics,
    number: u16,
    color: Rgba<u8>,
) {
    let digits: Vec<u8> = number
        .to_string()
        .bytes()
        .filter_map(|byte| byte.checked_sub(b'0'))
        .collect();
    let style = digit_style(digits.len());
    let digit_width = digit_width(style);
    let total_width =
        digits.len() as i32 * digit_width + (digits.len().saturating_sub(1) as i32 * style.gap);
    let start_x = ((ICON_SIZE - total_width) / 2).max(0);

    for (index, digit) in digits.iter().enumerate() {
        draw_digit(
            canvas,
            metrics,
            start_x + index as i32 * (digit_width + style.gap),
            style.y,
            *digit,
            style,
            color,
        );
    }
}

fn digit_style(digit_count: usize) -> DigitStyle {
    if digit_count >= 3 {
        DigitStyle {
            column_widths: [1, 1, 2, 1, 1],
            row_heights: DIGIT_ROW_HEIGHTS,
            y: DIGIT_Y,
            gap: 1,
        }
    } else if digit_count == 2 {
        DigitStyle {
            column_widths: [2, 1, 2, 1, 2],
            row_heights: DIGIT_ROW_HEIGHTS,
            y: DIGIT_Y,
            gap: 4,
        }
    } else {
        DigitStyle {
            column_widths: [2, 2, 2, 2, 2],
            row_heights: DIGIT_ROW_HEIGHTS,
            y: DIGIT_Y,
            gap: 0,
        }
    }
}

fn digit_width(style: DigitStyle) -> i32 {
    style.column_widths.iter().sum()
}

fn draw_digit(
    canvas: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    metrics: IconMetrics,
    x: i32,
    y: i32,
    digit: u8,
    style: DigitStyle,
    color: Rgba<u8>,
) {
    let Some(rows) = DIGIT_BITMAPS.get(digit as usize) else {
        return;
    };

    for (row_index, row) in rows.iter().enumerate() {
        let row_y = y + style.row_heights[..row_index].iter().sum::<i32>();

        for (column_index, cell) in row.as_bytes().iter().enumerate() {
            if *cell == b'1' {
                let column_x = x + style.column_widths[..column_index].iter().sum::<i32>();

                draw_rect(
                    canvas,
                    metrics,
                    column_x,
                    row_y,
                    style.column_widths[column_index],
                    style.row_heights[row_index],
                    color,
                );
            }
        }
    }
}

fn draw_rect(
    canvas: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    _metrics: IconMetrics,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    color: Rgba<u8>,
) {
    let max_x = canvas.width() as i32;
    let max_y = canvas.height() as i32;

    for py in y.max(0)..(y + height).min(max_y) {
        for px in x.max(0)..(x + width).min(max_x) {
            canvas.put_pixel(px as u32, py as u32, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_icon_uses_idle_background() {
        let render = TrayIconRender {
            bpm: None,
            state: IconState::Idle,
            tap_count: 0,
            stable_tap_dots: 24,
            theme: IconTheme::default(),
            metrics: IconMetrics::default(),
        };

        let canvas = render_icon_pixels(render);

        assert_eq!(canvas.width(), 20);
        assert_eq!(canvas.height(), 20);
        assert_eq!(*canvas.get_pixel(4, 12), Rgba([51, 65, 85, 255]));
        assert_eq!(*canvas.get_pixel(10, 5), Rgba([248, 250, 252, 255]));
        assert_eq!(*canvas.get_pixel(0, 0), Rgba([51, 65, 85, 255]));
    }

    #[test]
    fn idle_tray_icon_uses_embedded_asset() {
        let canvas = render_idle_icon_pixels().expect("expected idle icon asset to load");

        assert!(canvas.width() > ICON_SIZE as u32);
        assert!(canvas.height() > ICON_SIZE as u32);
    }

    #[test]
    fn collecting_icon_uses_lower_grid_indicators() {
        let render = TrayIconRender {
            bpm: None,
            state: IconState::Collecting,
            tap_count: 1,
            stable_tap_dots: 24,
            theme: IconTheme::default(),
            metrics: IconMetrics::default(),
        };

        let canvas = render_icon_pixels(render);

        let filled_cell_pixels = canvas
            .pixels()
            .filter(|pixel| **pixel == Rgba([64, 212, 255, 255]))
            .count();

        assert_eq!(*canvas.get_pixel(10, 5), Rgba([255, 255, 0, 255]));
        assert_eq!(*canvas.get_pixel(0, 12), Rgba([64, 212, 255, 255]));
        assert_eq!(*canvas.get_pixel(2, 12), Rgba([67, 67, 67, 255]));
        assert_eq!(*canvas.get_pixel(19, 19), Rgba([67, 67, 67, 255]));
        assert!(filled_cell_pixels >= 4);
    }

    #[test]
    fn eighth_state_connects_full_completed_row() {
        let render = TrayIconRender {
            bpm: None,
            state: IconState::Collecting,
            tap_count: 8,
            stable_tap_dots: 24,
            theme: IconTheme::default(),
            metrics: IconMetrics::default(),
        };

        let canvas = render_icon_pixels(render);

        assert_eq!(*canvas.get_pixel(2, 12), Rgba([64, 212, 255, 255]));
        assert_eq!(*canvas.get_pixel(2, 13), Rgba([64, 212, 255, 255]));
    }

    #[test]
    fn stable_full_grid_highlights_newest_cell() {
        let render = TrayIconRender {
            bpm: Some(120),
            state: IconState::Stable,
            tap_count: 24,
            stable_tap_dots: 24,
            theme: IconTheme::default(),
            metrics: IconMetrics::default(),
        };

        let canvas = render_icon_pixels(render);
        let newest_pixels = canvas
            .pixels()
            .filter(|pixel| **pixel == Rgba([64, 212, 255, 255]))
            .count();

        assert!(newest_pixels > 0);
    }

    #[test]
    fn twenty_third_tap_keeps_seven_cells_on_bottom_row() {
        let render = TrayIconRender {
            bpm: None,
            state: IconState::Collecting,
            tap_count: 23,
            stable_tap_dots: 24,
            theme: IconTheme::default(),
            metrics: IconMetrics::default(),
        };

        let canvas = render_icon_pixels(render);

        assert_eq!(*canvas.get_pixel(18, 18), Rgba([64, 212, 255, 255]));
        assert_eq!(*canvas.get_pixel(2, 18), Rgba([67, 67, 67, 255]));
    }

    #[test]
    fn full_grid_rolls_by_completed_rows_on_twenty_fourth_tap() {
        let render = TrayIconRender {
            bpm: None,
            state: IconState::Collecting,
            tap_count: 24,
            stable_tap_dots: 24,
            theme: IconTheme::default(),
            metrics: IconMetrics::default(),
        };

        let canvas = render_icon_pixels(render);

        assert_eq!(*canvas.get_pixel(0, 12), Rgba([64, 212, 255, 255]));
        assert_eq!(*canvas.get_pixel(2, 12), Rgba([64, 212, 255, 255]));
        assert_eq!(*canvas.get_pixel(0, 15), Rgba([64, 212, 255, 255]));
        assert_eq!(*canvas.get_pixel(2, 15), Rgba([64, 212, 255, 255]));
        assert_eq!(*canvas.get_pixel(0, 18), Rgba([67, 67, 67, 255]));
        assert_eq!(*canvas.get_pixel(2, 18), Rgba([67, 67, 67, 255]));
    }

    #[test]
    fn three_digit_numbers_are_drawn_inside_icon() {
        let render = TrayIconRender {
            bpm: Some(240),
            state: IconState::Stable,
            tap_count: 4,
            stable_tap_dots: 24,
            theme: IconTheme::default(),
            metrics: IconMetrics::default(),
        };

        let canvas = render_icon_pixels(render);
        let foreground_pixels = canvas
            .pixels()
            .filter(|pixel| **pixel == Rgba([255, 255, 0, 255]))
            .count();

        assert!(foreground_pixels > 30);
        assert_eq!(*canvas.get_pixel(0, 0), Rgba([255, 255, 0, 255]));
        assert_eq!(*canvas.get_pixel(1, 11), Rgba([67, 67, 67, 255]));
        assert_eq!(*canvas.get_pixel(19, 19), Rgba([67, 67, 67, 255]));
    }

    #[test]
    fn icon_uses_direct_twenty_unit_backing_resolution() {
        let render = TrayIconRender {
            bpm: Some(120),
            state: IconState::Stable,
            tap_count: 4,
            stable_tap_dots: 24,
            theme: IconTheme::default(),
            metrics: IconMetrics,
        };

        let canvas = render_icon_pixels(render);

        assert_eq!(canvas.width(), 20);
        assert_eq!(canvas.height(), 20);
    }
}

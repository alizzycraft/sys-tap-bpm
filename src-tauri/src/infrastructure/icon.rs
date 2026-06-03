use image::{ImageBuffer, ImageFormat, Rgba};
use tauri::image::Image;

const LOGICAL_ICON_SIZE: i32 = 64;
const DEFAULT_RENDER_SCALE: f32 = 2.0;
const MIN_RENDER_SCALE: f32 = 1.5;
const MAX_RENDER_SCALE: f32 = 2.5;
const BADGE_INSET: i32 = 3;
const BADGE_BOTTOM_OVERHANG: i32 = 3;
const INDICATOR_COLUMNS: usize = 5;
const INDICATOR_ROWS: usize = 3;
const INDICATOR_X: i32 = 8;
const INDICATOR_Y: i32 = 31;
const INDICATOR_CELL: i32 = 8;
const INDICATOR_GAP_X: i32 = 2;
const INDICATOR_GAP_Y: i32 = 1;

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

#[derive(Clone, Copy, Debug)]
pub struct IconMetrics {
    pub render_scale: f32,
}

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
                background: Rgba([37, 99, 235, 255]),
                foreground: Rgba([248, 250, 252, 255]),
                empty_dot: Rgba([30, 64, 175, 255]),
                filled_dot: Rgba([191, 219, 254, 255]),
                newest_dot: Rgba([250, 204, 21, 255]),
            },
            stable: IconColors {
                background: Rgba([22, 163, 74, 255]),
                foreground: Rgba([248, 250, 252, 255]),
                empty_dot: Rgba([20, 83, 45, 255]),
                filled_dot: Rgba([220, 252, 231, 255]),
                newest_dot: Rgba([250, 204, 21, 255]),
            },
        }
    }
}

impl Default for IconMetrics {
    fn default() -> Self {
        Self {
            render_scale: DEFAULT_RENDER_SCALE,
        }
    }
}

impl IconMetrics {
    fn scale(self) -> f32 {
        self.render_scale.clamp(MIN_RENDER_SCALE, MAX_RENDER_SCALE)
    }

    fn render_size(self) -> u32 {
        (LOGICAL_ICON_SIZE as f32 * self.scale()).round() as u32
    }

    fn px(self, logical: i32) -> i32 {
        (logical as f32 * self.scale()).round() as i32
    }

    fn px_len(self, logical: i32) -> i32 {
        (logical as f32 * self.scale()).round().max(1.0) as i32
    }
}

#[derive(Clone, Copy)]
struct DigitStyle {
    block_width: i32,
    block_height: i32,
    y: i32,
    gap: i32,
}

const DIGIT_COLUMNS: usize = 5;
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
    let canvas = render_icon_pixels(render);
    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(canvas)
        .write_to(&mut std::io::Cursor::new(&mut png), ImageFormat::Png)?;
    Image::from_bytes(&png).map_err(|error| {
        image::ImageError::IoError(std::io::Error::new(std::io::ErrorKind::Other, error))
    })
}

fn render_icon_pixels(render: TrayIconRender) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let colors = render.theme.colors_for(render.state);
    let metrics = render.metrics;
    let size = metrics.render_size();
    let mut canvas = ImageBuffer::from_pixel(size, size, Rgba([0, 0, 0, 0]));

    draw_rounded_rect(
        &mut canvas,
        metrics,
        BADGE_INSET,
        BADGE_INSET,
        LOGICAL_ICON_SIZE - BADGE_INSET * 2,
        LOGICAL_ICON_SIZE - BADGE_INSET * 2 + BADGE_BOTTOM_OVERHANG,
        7,
        colors.background,
    );
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

    let filled = tap_count.min(stable_tap_dots);
    let newest_index = tap_count.saturating_sub(1) % stable_tap_dots;

    for index in 0..stable_tap_dots {
        let color = if filled == stable_tap_dots && index == newest_index {
            newest_color
        } else if index < filled {
            filled_color
        } else {
            empty_color
        };

        draw_indicator_cell(canvas, metrics, index, color);
    }
}

fn draw_indicator_cell(
    canvas: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    metrics: IconMetrics,
    index: usize,
    color: Rgba<u8>,
) {
    let column = (index % INDICATOR_COLUMNS) as i32;
    let row = (index / INDICATOR_COLUMNS).min(INDICATOR_ROWS - 1) as i32;
    let x = INDICATOR_X + column * (INDICATOR_CELL + INDICATOR_GAP_X);
    let y = INDICATOR_Y + row * (INDICATOR_CELL + INDICATOR_GAP_Y);

    draw_rect(canvas, metrics, x, y, INDICATOR_CELL, INDICATOR_CELL, color);
}

fn draw_rounded_rect(
    canvas: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    metrics: IconMetrics,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    radius: i32,
    color: Rgba<u8>,
) {
    let x = metrics.px(x);
    let y = metrics.px(y);
    let width = metrics.px_len(width);
    let height = metrics.px_len(height);
    let radius = metrics.px_len(radius);
    let right = x + width - 1;
    let bottom = y + height - 1;
    let radius_squared = radius * radius;

    for py in y.max(0)..(y + height).min(canvas.height() as i32) {
        for px in x.max(0)..(x + width).min(canvas.width() as i32) {
            let corner_dx = if px < x + radius {
                x + radius - px
            } else if px > right - radius {
                px - (right - radius)
            } else {
                0
            };
            let corner_dy = if py < y + radius {
                y + radius - py
            } else if py > bottom - radius {
                py - (bottom - radius)
            } else {
                0
            };

            if corner_dx == 0
                || corner_dy == 0
                || corner_dx * corner_dx + corner_dy * corner_dy <= radius_squared
            {
                canvas.put_pixel(px as u32, py as u32, color);
            }
        }
    }
}

fn draw_tap_mark(
    canvas: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    metrics: IconMetrics,
    color: Rgba<u8>,
) {
    draw_rect(canvas, metrics, 20, 8, 24, 6, color);
    draw_rect(canvas, metrics, 29, 8, 6, 25, color);
    draw_rect(canvas, metrics, 22, 28, 20, 6, color);
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
    let digit_width = DIGIT_COLUMNS as i32 * style.block_width;
    let total_width =
        digits.len() as i32 * digit_width + (digits.len().saturating_sub(1) as i32 * style.gap);
    let start_x = ((64 - total_width) / 2).max(3);

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
            block_width: 3,
            block_height: 4,
            y: 1,
            gap: 2,
        }
    } else if digit_count == 2 {
        DigitStyle {
            block_width: 4,
            block_height: 4,
            y: 3,
            gap: 4,
        }
    } else {
        DigitStyle {
            block_width: 5,
            block_height: 5,
            y: 4,
            gap: 0,
        }
    }
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
        for (column_index, cell) in row.as_bytes().iter().enumerate() {
            if *cell == b'1' {
                draw_rect(
                    canvas,
                    metrics,
                    x + column_index as i32 * style.block_width,
                    y + row_index as i32 * style.block_height,
                    style.block_width,
                    style.block_height,
                    color,
                );
            }
        }
    }
}

fn draw_rect(
    canvas: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    metrics: IconMetrics,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    color: Rgba<u8>,
) {
    let x = metrics.px(x);
    let y = metrics.px(y);
    let width = metrics.px_len(width);
    let height = metrics.px_len(height);
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
            stable_tap_dots: 15,
            theme: IconTheme::default(),
            metrics: IconMetrics::default(),
        };

        let canvas = render_icon_pixels(render);

        assert_eq!(canvas.width(), 128);
        assert_eq!(canvas.height(), 128);
        assert_eq!(*canvas.get_pixel(10, 64), Rgba([51, 65, 85, 255]));
        assert_eq!(*canvas.get_pixel(64, 64), Rgba([248, 250, 252, 255]));
        assert_eq!(*canvas.get_pixel(0, 0), Rgba([0, 0, 0, 0]));
    }

    #[test]
    fn collecting_icon_uses_lower_grid_indicators() {
        let render = TrayIconRender {
            bpm: None,
            state: IconState::Collecting,
            tap_count: 1,
            stable_tap_dots: 15,
            theme: IconTheme::default(),
            metrics: IconMetrics::default(),
        };

        let canvas = render_icon_pixels(render);

        let filled_cell_pixels = canvas
            .pixels()
            .filter(|pixel| **pixel == Rgba([191, 219, 254, 255]))
            .count();

        assert_eq!(*canvas.get_pixel(64, 64), Rgba([248, 250, 252, 255]));
        assert_eq!(*canvas.get_pixel(16, 64), Rgba([191, 219, 254, 255]));
        assert!(filled_cell_pixels > 200);
    }

    #[test]
    fn stable_full_grid_highlights_newest_cell() {
        let render = TrayIconRender {
            bpm: Some(120),
            state: IconState::Stable,
            tap_count: 15,
            stable_tap_dots: 15,
            theme: IconTheme::default(),
            metrics: IconMetrics::default(),
        };

        let canvas = render_icon_pixels(render);
        let newest_pixels = canvas
            .pixels()
            .filter(|pixel| **pixel == Rgba([250, 204, 21, 255]))
            .count();

        assert!(newest_pixels > 0);
    }

    #[test]
    fn three_digit_numbers_are_drawn_inside_icon() {
        let render = TrayIconRender {
            bpm: Some(240),
            state: IconState::Stable,
            tap_count: 4,
            stable_tap_dots: 15,
            theme: IconTheme::default(),
            metrics: IconMetrics::default(),
        };

        let canvas = render_icon_pixels(render);
        let foreground_pixels = canvas
            .pixels()
            .filter(|pixel| **pixel == Rgba([248, 250, 252, 255]))
            .count();

        assert!(foreground_pixels > 100);
        assert_eq!(*canvas.get_pixel(0, 0), Rgba([0, 0, 0, 0]));
        assert_eq!(*canvas.get_pixel(127, 127), Rgba([0, 0, 0, 0]));
    }

    #[test]
    fn custom_render_scale_changes_backing_resolution() {
        let render = TrayIconRender {
            bpm: Some(120),
            state: IconState::Stable,
            tap_count: 4,
            stable_tap_dots: 15,
            theme: IconTheme::default(),
            metrics: IconMetrics { render_scale: 2.5 },
        };

        let canvas = render_icon_pixels(render);

        assert_eq!(canvas.width(), 160);
        assert_eq!(canvas.height(), 160);
    }
}

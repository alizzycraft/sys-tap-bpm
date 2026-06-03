use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

mod domain;

use domain::tap_tempo::{TapTempoConfig, TapTempoSession, TapTempoSnapshot};
use image::{ImageBuffer, ImageFormat, Rgba};
use serde::Serialize;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, PhysicalPosition, Position,
};

const RESET_AFTER: Duration = Duration::from_secs(3);
const AVERAGING_WINDOW: Duration = Duration::from_secs(45);
const MIN_INTERVALS: usize = 2;
const STABLE_TAP_DOTS: usize = 24;
const FLOATING_LABEL: &str = "floating";

#[derive(Clone, Copy)]
enum DisplayMode {
    IconAndFloating,
}

#[derive(Clone)]
struct AppConfig {
    tap_tempo: TapTempoConfig,
    stable_tap_dots: usize,
    display_mode: DisplayMode,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            tap_tempo: TapTempoConfig {
                reset_after: RESET_AFTER,
                averaging_window: AVERAGING_WINDOW,
                min_intervals: MIN_INTERVALS,
            },
            stable_tap_dots: STABLE_TAP_DOTS,
            display_mode: DisplayMode::IconAndFloating,
        }
    }
}

#[derive(Serialize, Clone)]
struct BpmUpdate {
    bpm: Option<f64>,
    #[serde(rename = "tapCount")]
    tap_count: usize,
    #[serde(rename = "isActive")]
    is_active: bool,
    #[serde(rename = "displayMode")]
    display_mode: &'static str,
}

struct SharedState {
    config: AppConfig,
    tap_tempo: Mutex<TapTempoSession>,
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let state = Arc::new(SharedState {
                config: AppConfig::default(),
                tap_tempo: Mutex::new(TapTempoSession::default()),
            });

            app.manage(state.clone());
            create_tray(app.handle(), state)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running sys-tap-bpm");
}

fn create_tray(app: &AppHandle, state: Arc<SharedState>) -> tauri::Result<TrayIcon> {
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&quit])?;
    let icon = render_icon(None, false, 0, state.config.stable_tap_dots)
        .map_err(|error| tauri::Error::Anyhow(error.into()))?;

    TrayIconBuilder::new()
        .icon(icon)
        .tooltip("sys-tap-bpm: tap to start")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if event.id().as_ref() == "quit" {
                app.exit(0);
            }
        })
        .on_tray_icon_event(move |tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                let app = tray.app_handle();
                let (x, y) = match rect.position {
                    Position::Physical(position) => (position.x as f64, position.y as f64),
                    Position::Logical(position) => (position.x, position.y),
                };
                handle_tap(&app, tray, &state, x, y);
            }
        })
        .build(app)
}

fn handle_tap(app: &AppHandle, tray: &TrayIcon, state: &Arc<SharedState>, x: f64, y: f64) {
    let update = {
        let mut session = state.tap_tempo.lock().expect("tap tempo state poisoned");
        let snapshot = session.tap(Instant::now(), &state.config.tap_tempo);
        current_update(snapshot, state.config.display_mode)
    };

    publish_update(app, tray, &update, Some((x, y)));
    schedule_reset(app.clone(), tray.clone(), state.clone());
}

fn schedule_reset(app: AppHandle, tray: TrayIcon, state: Arc<SharedState>) {
    std::thread::spawn(move || {
        std::thread::sleep(state.config.tap_tempo.reset_after);

        let update = {
            let mut session = state.tap_tempo.lock().expect("tap tempo state poisoned");
            let Some(snapshot) =
                session.reset_if_inactive(Instant::now(), state.config.tap_tempo.reset_after)
            else {
                return;
            };

            current_update(snapshot, state.config.display_mode)
        };

        publish_update(&app, &tray, &update, None);
    });
}

fn current_update(snapshot: TapTempoSnapshot, display_mode: DisplayMode) -> BpmUpdate {
    BpmUpdate {
        bpm: snapshot.bpm,
        tap_count: snapshot.tap_count,
        is_active: snapshot.is_active,
        display_mode: match display_mode {
            DisplayMode::IconAndFloating => "icon-and-floating",
        },
    }
}

fn publish_update(
    app: &AppHandle,
    tray: &TrayIcon,
    update: &BpmUpdate,
    position: Option<(f64, f64)>,
) {
    let rounded_bpm = update.bpm.map(|bpm| bpm.round() as u16);
    let tooltip = match rounded_bpm {
        Some(bpm) => format!("sys-tap-bpm: {bpm} BPM"),
        None if update.is_active => "sys-tap-bpm: listening...".to_string(),
        None => "sys-tap-bpm: tap to start".to_string(),
    };

    if let Ok(icon) = render_icon(
        rounded_bpm,
        update.is_active,
        update.tap_count,
        STABLE_TAP_DOTS,
    ) {
        let _ = tray.set_icon(Some(icon));
    }

    let _ = tray.set_tooltip(Some(&tooltip));
    let _ = app.emit_to(FLOATING_LABEL, "bpm-update", update);

    if let Some(window) = app.get_webview_window(FLOATING_LABEL) {
        if update.is_active {
            if let Some((x, y)) = position {
                let position =
                    Position::Physical(PhysicalPosition::new(x as i32 - 180, y as i32 - 118));
                let _ = window.set_position(position);
            }

            let _ = window.show();
            let _ = window.set_focus();
        } else {
            let _ = window.hide();
        }
    }
}

fn render_icon(
    bpm: Option<u16>,
    active: bool,
    tap_count: usize,
    stable_tap_dots: usize,
) -> image::ImageResult<Image<'static>> {
    let background = if active {
        Rgba([22, 163, 74, 255])
    } else {
        Rgba([71, 85, 105, 255])
    };
    let foreground = Rgba([248, 250, 252, 255]);
    let muted_dot = if active {
        Rgba([20, 83, 45, 255])
    } else {
        Rgba([51, 65, 85, 255])
    };
    let dot = Rgba([248, 250, 252, 255]);
    let newest_dot = Rgba([250, 204, 21, 255]);
    let mut canvas = ImageBuffer::from_pixel(64, 64, Rgba([0, 0, 0, 0]));

    draw_circle(&mut canvas, 32, 32, 29, background);
    draw_tap_dots(
        &mut canvas,
        tap_count,
        stable_tap_dots,
        if active { muted_dot } else { background },
        dot,
        newest_dot,
    );

    match bpm {
        Some(value) => draw_centered_number(&mut canvas, value.min(999), foreground),
        None => draw_tap_mark(&mut canvas, foreground),
    }

    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(canvas)
        .write_to(&mut std::io::Cursor::new(&mut png), ImageFormat::Png)?;
    Image::from_bytes(&png).map_err(|error| {
        image::ImageError::IoError(std::io::Error::new(std::io::ErrorKind::Other, error))
    })
}

fn draw_tap_dots(
    canvas: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
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
        let angle = -std::f64::consts::FRAC_PI_2
            + (index as f64 / stable_tap_dots as f64) * std::f64::consts::TAU;
        let x = 32 + (angle.cos() * 27.0).round() as i32;
        let y = 32 + (angle.sin() * 27.0).round() as i32;
        let color = if filled == stable_tap_dots && index == newest_index {
            newest_color
        } else if index < filled {
            filled_color
        } else {
            empty_color
        };

        draw_circle(canvas, x, y, 2, color);
    }
}

fn draw_circle(
    canvas: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    cx: i32,
    cy: i32,
    radius: i32,
    color: Rgba<u8>,
) {
    let radius_squared = radius * radius;

    for y in 0..64 {
        for x in 0..64 {
            let dx = x as i32 - cx;
            let dy = y as i32 - cy;

            if dx * dx + dy * dy <= radius_squared {
                canvas.put_pixel(x, y, color);
            }
        }
    }
}

fn draw_tap_mark(canvas: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, color: Rgba<u8>) {
    draw_rect(canvas, 18, 16, 28, 7, color);
    draw_rect(canvas, 28, 16, 8, 32, color);
    draw_rect(canvas, 20, 41, 24, 7, color);
}

fn draw_centered_number(canvas: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, number: u16, color: Rgba<u8>) {
    let digits: Vec<u8> = number
        .to_string()
        .bytes()
        .filter_map(|byte| byte.checked_sub(b'0'))
        .collect();
    let digit_width = 13;
    let gap = 3;
    let total_width =
        digits.len() as i32 * digit_width + (digits.len().saturating_sub(1) as i32 * gap);
    let start_x = ((64 - total_width) / 2).max(3);

    for (index, digit) in digits.iter().enumerate() {
        draw_digit(
            canvas,
            start_x + index as i32 * (digit_width + gap),
            18,
            *digit,
            color,
        );
    }
}

fn draw_digit(
    canvas: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    x: i32,
    y: i32,
    digit: u8,
    color: Rgba<u8>,
) {
    const SEGMENTS: [[bool; 7]; 10] = [
        [true, true, true, true, true, true, false],
        [false, true, true, false, false, false, false],
        [true, true, false, true, true, false, true],
        [true, true, true, true, false, false, true],
        [false, true, true, false, false, true, true],
        [true, false, true, true, false, true, true],
        [true, false, true, true, true, true, true],
        [true, true, true, false, false, false, false],
        [true, true, true, true, true, true, true],
        [true, true, true, true, false, true, true],
    ];

    let Some(segments) = SEGMENTS.get(digit as usize) else {
        return;
    };

    if segments[0] {
        draw_rect(canvas, x + 2, y, 9, 4, color);
    }
    if segments[1] {
        draw_rect(canvas, x + 9, y + 3, 4, 11, color);
    }
    if segments[2] {
        draw_rect(canvas, x + 9, y + 17, 4, 11, color);
    }
    if segments[3] {
        draw_rect(canvas, x + 2, y + 28, 9, 4, color);
    }
    if segments[4] {
        draw_rect(canvas, x, y + 17, 4, 11, color);
    }
    if segments[5] {
        draw_rect(canvas, x, y + 3, 4, 11, color);
    }
    if segments[6] {
        draw_rect(canvas, x + 2, y + 14, 9, 4, color);
    }
}

fn draw_rect(
    canvas: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    color: Rgba<u8>,
) {
    for py in y.max(0)..(y + height).min(64) {
        for px in x.max(0)..(x + width).min(64) {
            canvas.put_pixel(px as u32, py as u32, color);
        }
    }
}

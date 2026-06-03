use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

mod domain;
mod infrastructure;

use domain::tap_tempo::{TapTempoConfig, TapTempoSession, TapTempoSnapshot};
use infrastructure::icon::{render_tray_icon, IconMetrics, IconState, IconTheme, TrayIconRender};
use serde::Serialize;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, PhysicalPosition, Position,
};

const RESET_AFTER: Duration = Duration::from_secs(3);
const AVERAGING_WINDOW: Duration = Duration::from_secs(30);
const MIN_INTERVALS: usize = 2;
const STABLE_TAP_DOTS: usize = 15;
const FLOATING_LABEL: &str = "floating";

#[derive(Clone, Copy)]
enum DisplayMode {
    IconAndFloating,
}

#[derive(Clone)]
struct AppConfig {
    tap_tempo: TapTempoConfig,
    stable_tap_dots: usize,
    icon_theme: IconTheme,
    icon_metrics: IconMetrics,
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
            icon_theme: IconTheme::default(),
            icon_metrics: IconMetrics::default(),
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
    let icon = render_tray_icon(TrayIconRender {
        bpm: None,
        state: IconState::Idle,
        tap_count: 0,
        stable_tap_dots: state.config.stable_tap_dots,
        theme: state.config.icon_theme,
        metrics: state.config.icon_metrics,
    })
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

    publish_update(app, tray, &state.config, &update, Some((x, y)));
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

        publish_update(&app, &tray, &state.config, &update, None);
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
    config: &AppConfig,
    update: &BpmUpdate,
    position: Option<(f64, f64)>,
) {
    let rounded_bpm = update.bpm.map(|bpm| bpm.round() as u16);
    let tooltip = match rounded_bpm {
        Some(bpm) => format!("sys-tap-bpm: {bpm} BPM"),
        None if update.is_active => "sys-tap-bpm: listening...".to_string(),
        None => "sys-tap-bpm: tap to start".to_string(),
    };

    if let Ok(icon) = render_tray_icon(TrayIconRender {
        bpm: rounded_bpm,
        state: icon_state(update),
        tap_count: update.tap_count,
        stable_tap_dots: config.stable_tap_dots,
        theme: config.icon_theme,
        metrics: config.icon_metrics,
    }) {
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

fn icon_state(update: &BpmUpdate) -> IconState {
    if update.bpm.is_some() {
        IconState::Stable
    } else if update.is_active {
        IconState::Collecting
    } else {
        IconState::Idle
    }
}

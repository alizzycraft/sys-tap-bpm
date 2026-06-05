# sys-tap-bpm Project Plan

## Goal

Build `sys-tap-bpm` into a small, reliable tray-first tap tempo app that can later serve as a base for richer system tray experiences, including a tray-based MUD game.

The first product target is a BPM utility that stays idle most of the time, reacts instantly to tray clicks, displays a stable tempo estimate, and remains lightweight enough to feel native.

## Current State

- Tauri 2 app scaffold exists.
- Angular 21 frontend exists with standalone component, zoneless change detection, `signal`, and `computed`.
- Tray clicks are handled in Rust.
- BPM is calculated from recent tap intervals.
- Tapping resets after 3 seconds of inactivity.
- Tray tooltip and dynamic tray icon are wired.
- Floating Angular UI exists but is currently hidden/disabled while the tray-first experience is refined.
- Idle tray state uses a dedicated bitmap asset from `assets/tray/idle.png`; active tapping states use the generated BPM icon.
- Tray icon can render BPM above a 15-cell tap indicator grid.
- Frontend builds pass with `npm run check` and `npm run build`.
- Local Tauri prerequisite checks pass with Rust, Cargo/rustup, MSVC, Windows SDK, and WebView2 detected.
- `npm run tauri:dev` launches and the tray icon appears.

## Product Shape

### Primary Use Case

1. App starts hidden in the system tray.
2. User taps the tray icon repeatedly in time with audio or a rhythm.
3. App records tap intervals and calculates BPM from the recent rolling window.
4. App gives visual feedback through the tray icon, tooltip, and optional floating UI.
5. If the user stops tapping for 3 seconds, the app resets to idle.

### Display Modes

Support these display modes as configuration, even if the first version ships with only one or two enabled:

- `tooltip`: BPM appears in the tray tooltip.
- `icon`: BPM appears inside the tray icon.
- `floating`: BPM appears in a small always-on-top floating window near the tray.
- `icon-and-floating`: both tray icon and floating UI update while tapping.

## Architecture Direction

Use a skimmed-down onion architecture that keeps the app small while protecting the core tap-tempo behavior from UI and native-host details.

Initial layering:

- Domain core: pure tap-tempo logic, including tap sessions, interval averaging, inactivity reset rules, rolling-window pruning, and BPM snapshots. This layer must not depend on Tauri, Angular, image rendering, filesystem access, or timers.
- Application/native host: coordinates user actions such as tray clicks and reset checks, owns shared app state, and converts domain snapshots into app updates.
- Infrastructure/adapters: Tauri tray wiring, window behavior, icon rendering, reset timer scheduling, config persistence, and OS integration.
- Presentation: Angular floating UI, signal state, and computed display values.

Do not create empty architecture folders before they carry real behavior. Start Phase 2 by extracting only the tap-tempo domain core into a testable Rust module. Split tray, icon, config, and Angular state into separate modules later when their corresponding phases need them.

## Implementation Phases

## Phase 1: Runtime Prerequisites

Status: Complete.

Goal: Make the project runnable locally.

Tasks:

- Install Rust with rustup.
- Install Visual Studio Build Tools with MSVC and Windows SDK.
- Confirm `npm run tauri -- info` has no required environment failures.
- Run `npm run tauri:dev`.
- Verify the app appears in the system tray.
- Verify quit menu behavior.

Acceptance criteria:

- `npm run tauri:dev` launches the app.
- Tray icon is visible.
- App exits cleanly from the tray menu.

## Phase 2: Tap Tempo Correctness

Status: Complete. Optional outlier rejection is deferred until accidental tap handling becomes a real UX problem.

Goal: Make BPM calculation predictable and testable.

Tasks:

- Move tap timing/BPM calculation into a small Rust module separate from tray/UI code.
- Add unit tests for interval averaging.
- Add tests for reset behavior after inactivity.
- Add tests for rolling window behavior.
- Use the last 10 seconds as the default averaging window.
- Defer optional outlier rejection for accidental late/early taps.

Implementation notes:

- Estimate BPM with a least-squares tempo line across tap timestamps rather than direct adjacent-interval averaging, so repeated taps settle more cleanly under small human timing jitter.
- Keep raw BPM separate from stabilized display BPM; publish the stabilized integer BPM to avoid single-digit flicker around rounding boundaries.
- Show `0` on the first tap, then a provisional display BPM as soon as one interval exists, even before the raw BPM meets the configured minimum interval threshold.
- Require a minimum number of intervals before showing a BPM.
- Keep the first tap as session start, not as a BPM-producing input.
- Default averaging window is 10 seconds.

Acceptance criteria:

- BPM tests cover steady taps, changing tempo, inactivity reset, and stale tap pruning.
- The displayed BPM does not jump on the first or second tap unless intentionally configured.

## Phase 3: Tray Icon Feedback

Status: Implementation complete. Manual readability verification in the real Windows tray is still required before closing acceptance.

Goal: Make the tray icon useful at a glance.

Tasks:

- Refine the dynamic icon renderer.
- Confirm 24-state lower tap indicator grid readability at real tray sizes.
- Decide how filled indicators should behave after the grid is full:
  - Option A: keep full indicator and highlight newest cell. Selected.
  - Option B: show moving recent-tap trail.
  - Option C: age dots by brightness.
- Improve number rendering for 2-digit and 3-digit BPM values.
- Add icon colors for idle, tapping, stable, and reset states.
- Keep active tapping and stable reads on the same icon color treatment; use the numbers and indicators rather than color changes for tactile feedback.
- Active icon colors are `#434343` background, `#ffff00` BPM digits, and `#40d4ff` filled indicators.
- Consider rendering separate scale variants if Windows blurs the generated PNG.
- Keep icon colors isolated behind a theme structure so future settings can customize icon elements without rewriting the renderer.
- Render icon graphics directly into a 20x20 unit backing image so each layout unit maps to one displayed tray pixel and indicator gaps stay crisp.
- Use a full square tray badge to give the stability indicator and BPM digits the full icon area at real tray sizes.
- Render the stability indicator as 3 rows of 7 rectangular cells below the BPM digits, with an 8th state per row shown by filling connector gaps so the completed row becomes a solid bar.
- Default stability indicator count is 24 states: 8 states per row across 3 rows.
- On the 24th indicator state, roll the visible indicator window by whole 8-state rows: drop the oldest completed row, keep the two newer completed rows, and leave the bottom row empty for the next group.
- Use larger cell and digit stroke weights so tray downscaling does not reduce key feedback to single-pixel details.
- Render BPM digits with bold filled bitmap numerals instead of seven-segment calculator-style strokes.
- Use separate bitmap digit width and height so BPM digits can be taller without making 3-digit values overflow horizontally.
- Keep indicator cells vertically tighter than horizontally to preserve room for taller BPM digits.
- Use a full square background with no rounded-corner transparency so every available tray pixel can carry icon information.
- Use 2x2 indicator cells with 1-unit horizontal and vertical gaps so the lower grid has no left, right, or bottom margin.
- Place the BPM digits in the top 11 units, keep a 1-unit gap, and place the 8-unit indicator grid from unit 12 through unit 19.
- Use a 1-unit gap between 3-digit BPM numerals to avoid digits merging.

Acceptance criteria:

- Icon is readable in the Windows tray.
- User can tell whether the app is idle, collecting taps, or showing a stable read.
- BPM values from roughly 40 to 240 are legible enough to be useful.

## Phase 4: Floating UI

Status: Deferred while the tray-only experience is refined.

Goal: Make the floating Angular window feel deliberate rather than like a generic popup.

Tasks:

- Position the floating window near the tray more reliably.
- Avoid stealing focus unless explicitly useful.
- Hide the floating window after reset, or fade it out after a short delay.
- Add visual states:
  - idle
  - collecting
  - stable
  - reset
- Add optional secondary text for tap count or confidence.
- Ensure text never overflows the small window.

Acceptance criteria:

- Floating UI appears in a consistent location when tapping.
- Floating UI does not interrupt normal desktop work.
- BPM is readable at a glance.

## Phase 5: Configuration

Goal: Make behavior adjustable without hardcoding.

Initial settings:

- `displayMode`
- `resetAfterMs`
- `averagingWindowMs`
- `minIntervals`
- `stableTapDots`
- `idleColor`
- `activeColor`
- `stableColor`
- `showFloatingWindow`
- `showBpmInIcon`

Tasks:

- Define a shared config model.
- Persist config through Tauri storage or an app config file.
- Load config at startup.
- Add safe defaults if config is missing or invalid.
- Add a minimal settings window or tray submenu.

Acceptance criteria:

- Settings persist across app restarts.
- Invalid settings fall back safely.
- Runtime code uses config values instead of constants where appropriate.

## Phase 6: Angular State Architecture

Goal: Keep the frontend small now, but structured enough for future tray-based game UI.

Tasks:

- Create a typed app state service using Angular signals.
- Move Tauri event subscription out of the component.
- Add computed selectors for BPM text, tap text, confidence, and visual state.
- Keep components standalone.
- Keep zoneless change detection.
- Add a small message/event model that can later support richer tray UI events.

Suggested shape:

```ts
type AppEvent =
  | { type: "bpm-update"; payload: BpmUpdate }
  | { type: "config-update"; payload: AppConfig }
  | { type: "session-reset" };
```

Acceptance criteria:

- Components only render state.
- Tauri event wiring is isolated in one service.
- Adding a settings view does not require rewriting the BPM display.

## Phase 7: Packaging And Startup

Goal: Make the app installable and practical as a tray utility.

Tasks:

- Confirm Windows bundle builds.
- Add app icon assets required by Tauri bundling.
- Decide whether app should launch on startup.
- Add optional auto-start support.
- Verify app does not show a taskbar item.
- Verify app starts hidden.
- Verify app exits cleanly.

Acceptance criteria:

- A Windows installer or executable can be produced.
- Installed app runs as a tray utility.
- No unwanted main window appears at startup.

## Phase 8: QA Checklist

Manual test cases:

- App starts with tray icon visible.
- Single tap shows collecting state but no unstable BPM if below minimum interval count.
- Repeated steady taps converge to expected BPM.
- Tapping at 120 BPM shows approximately 120 BPM.
- Stopping for 3 seconds resets to idle.
- Floating window appears and hides according to config.
- Tooltip updates while tapping.
- Tray icon grid cells fill as taps are collected.
- Tray menu quit exits the app.
- App can be restarted repeatedly without stale windows or tray icons.

Automated test targets:

- Rust unit tests for BPM logic.
- Rust tests for config validation.
- Angular tests for signal/computed state derivation if the frontend grows.

## Phase 9: Future Tray MUD Foundation

Goal: Preserve a path from BPM utility to richer tray-based interaction.

Architectural choices to keep:

- Tauri owns native tray/window behavior.
- Angular owns reactive UI state and rendering.
- Event payloads are typed and explicit.
- UI state is modeled as signals and computed selectors.
- Tray actions are treated as input events, not hardcoded one-off callbacks.

Potential future additions:

- Tray menu as game command surface.
- Floating window as compact game status HUD.
- Notifications for events, combat, messages, or timers.
- Background simulation loop in Rust or TypeScript.
- Persisted game state.
- Multiple small windows for map, log, inventory, or prompt.

Do not build these into the BPM app yet. The immediate goal is to structure the app so this future direction stays cheap to explore.

## Suggested Next Step

Complete Phase 1 first by installing the native prerequisites and running the app. Once the tray behavior is verified on the real desktop, proceed to Phase 2 and extract/test the BPM calculation logic.

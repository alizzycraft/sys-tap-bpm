# sys-tap-bpm

A small Tauri + Angular 21 system tray tap-tempo utility.

## Behavior

- Runs from the system tray.
- Left-click the tray icon repeatedly to tap a tempo.
- The app estimates BPM from taps in the latest 10 seconds.
- If tapping stops for 3 seconds, the session resets.
- BPM is shown in the tray tooltip and tray icon.
- The tray icon uses a 24-state lower indicator grid: 3 rows of 8 tap states. Completed rows become solid bars, and the visible indicator window rolls by whole rows as tapping continues.

## Development

Install dependencies:

```powershell
npm install
```

Run the TypeScript check:

```powershell
npm run check
```

This runs an Angular development build with template/type checking.

Run the Tauri app:

```powershell
npm run tauri:dev
```

This requires the Rust toolchain and Tauri prerequisites to be installed.

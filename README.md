# sys-tap-bpm

A small Tauri + Angular 21 system tray tap-tempo utility.

## Behavior

- Runs from the system tray.
- Left-click the tray icon repeatedly to tap a tempo.
- The app averages tap intervals from the latest 45 seconds.
- If tapping stops for 3 seconds, the session resets.
- BPM is shown in the tray tooltip, in the tray icon, and in a small floating window.
- The tray icon border has 24 dots to show how many taps are contributing to the current read; once full, the newest tap advances around the ring.

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

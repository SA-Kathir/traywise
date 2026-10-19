# Traywise

One tray icon, one glassy panel, and a handful of small Windows 11 tools you can switch on or off.

<!-- PLACEHOLDER: replace with a real screenshot or GIF of the panel, e.g. docs/screenshot.png -->
![Traywise panel (screenshot coming soon)](docs/screenshot.png)

## Modules

Each module can be switched on or off in Settings. A module that's off doesn't run at all.

- **Volume mixer**: the volume and mute of each app that's playing sound, plus the master volume.
- **Audio devices**: pick the default speakers or headphones in one click, and mute every microphone at once (also with a shortcut, Ctrl+Alt+M by default).
- **Keep awake**: stop the PC from sleeping, until you turn it off, for a set time, or while plugged in. Can keep the screen on too.
- **System monitor**: CPU, memory, network speed, battery and free disk space, with a 60-second graph. It only measures while the panel is open.
- **Quick toggles**: switch light/dark mode, show or hide desktop icons, and lock the screen.

Left-click the tray icon to open the panel; right-click it to quit.

## Install

1. Download `Traywise_x.y.z_x64-setup.exe` from the [latest release](https://github.com/SA-Kathir/traywise/releases/latest).
2. Run it. It installs for your user only, so it doesn't ask for admin rights.
3. **Windows SmartScreen will warn you** ("Windows protected your PC"), because Traywise isn't code-signed yet. Click **More info**, then **Run anyway**. Signing is planned.

Traywise updates itself: it checks for a new version when it starts and once a day, and asks before installing. You can turn the checks off in Settings.

To uninstall, use **Settings → Apps → Installed apps → Traywise**.

## Requirements

- Windows 11 (64-bit).
- WebView2, which is built into Windows 11.

## Privacy

Traywise works locally. It has no accounts, no telemetry and no analytics, and it never reads your screen or wallpaper.

The only thing it sends over the network is the update check: a request to GitHub for the latest version's details, and a download if you choose to install. You can turn this off in Settings.

Your settings are saved in `%APPDATA%\Traywise\settings.json`.

## Build from source

You need [Node.js](https://nodejs.org/) 22+, [pnpm](https://pnpm.io/), [Rust](https://rustup.rs/) (stable) and the [Tauri prerequisites for Windows](https://tauri.app/start/prerequisites/).

```bash
pnpm install
pnpm tauri dev
```

`pnpm tauri dev` runs it with hot reload. For a release build, run `pnpm tauri build`; the installer ends up in `src-tauri/target/release/bundle/nsis/`.

See [CONTRIBUTING.md](CONTRIBUTING.md) for how the code is laid out.

## Licence

Traywise is free software under the [GNU GPL v3.0 or later](LICENSE).

Inspired by vorssaint-utils for macOS. Traywise is a separate project and shares no code with it.

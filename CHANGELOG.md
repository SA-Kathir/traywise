# Changelog

## Unreleased

- **A customisable home board.** Home is now a grid of tiles you arrange yourself:
  - Each module offers tiles in several sizes (1×1, 2×1, 4×1, 2×2, 4×2), and bigger ones do more. For example, the mixer goes from a mute button up to the master volume plus three apps, and Keep awake's 4×2 has its modes and timer.
  - Edit with the pencil button, or hold a tile. Drag tiles (or move them with the arrow keys), change their size, remove them, or add more. Reset goes back to the default board.
  - **Auto-arrange** fills every gap. Switch it off to keep tiles where you put them; they still move up to close space above them.
  - Your layout is saved. Switching a module on adds its tiles; switching it off removes them.
  - Updating from 0.1.0 gives you the new default board.
- The panel no longer nudges itself when a setting other than its look is saved.

## 0.1.0 (2026-10-26)

The first release.

- One tray icon and one panel, with two looks to pick from on first start (and in Settings):
  - **Liquid**: tiles tinted in one of six colour presets, on a tinted background or floating over the desktop, shaded or in a single hue.
  - **Windows**: flat, like Windows 11's own flyouts; follows light and dark mode.
- Home shows quick tiles, volume and a system summary; tabs (Sound, System) hold each module's full controls.
- A Modules page lists what each module does and what it costs while idle, with an All switch.
- Modules, each switched on or off in Settings → Modules:
  - **Volume mixer**: per-app and master volume and mute.
  - **Audio devices**: pick the default output; mute every microphone, with a shortcut.
  - **Keep awake**: until turned off, for a set time, or while plugged in; optionally keeps the screen on.
  - **System monitor**: CPU, memory, network, battery and disk, while the panel is open.
  - **Quick toggles**: dark mode, desktop icons, lock screen.
- A keyboard shortcut to open or close the panel, Ctrl+Alt+T by default (Settings → Shortcuts).
- Start with Windows.
- Auto-updates: checks GitHub at startup and about once a day, installs only when you click. Can be switched off.
- Per-user installer, no admin rights needed. It isn't code-signed yet, so Windows SmartScreen warns on first install.

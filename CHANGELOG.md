# Changelog

## 0.2.0 (2026-10-31)

### Highlights

- **A new look, Dot, the default for new installs.** Monochrome frosted tiles over a dot grid, numbers in a dot-matrix font, words in OCR-A, and one accent colour (Red, Amber, Violet, Blue or Green). Switched-on tiles turn into a solid plate with a small accent dot. It follows Windows' light or dark mode, and its background can be Dots, Plain or Transparent; Transparent glass gets more solid over dark wallpapers. Existing installs keep their look; Liquid and Windows are still there.
- **A customisable home board.** Home is now a grid of tiles you arrange yourself:
  - Tiles come in sizes from 1×1 to 4×2, and bigger ones show more. Their icons show their state (a moon or a sun for dark mode, a crossed-out eye for hidden desktop icons).
  - Edit with the pencil button, or hold a tile: drag (or use the arrow keys), resize, remove, or add tiles from a tray with a size picker. Reset goes back to the default board.
  - **Auto-arrange** fills every gap; switch it off to keep tiles where you put them.
  - Your layout is saved. Switching a module on adds its tiles; switching it off removes them. Updating from 0.1.0 gives you the new default board.
- **New modules:**
  - **Now playing**: what any app is playing, with cover art (a browser shows its own icon), play/pause, skip, a seek bar, shuffle and repeat when the app allows them. The Sound tab lists every app with media.
  - **Wi-Fi**: switch it, see the network you're on, and join another, typing a new network's password right there (it goes straight to Windows).
  - **Bluetooth**: switch it, see your paired devices, and connect or disconnect headphones and speakers.
  - Both have their own **Connect** tab, and tiles up to 2×2 with the network or device name.
- **Open the panel from the right edge of the screen** (Settings › Opening): rest the mouse on the middle of the edge and it slides in, then slides away when you move off. Optional small handle.

### Smaller changes

- System monitor: separate CPU, Memory, Download and Upload tiles, and a wide tile with any two of them (picked on the System page). Graphs zoom in to what they show, so a quiet CPU's ups and downs are visible.
- Volume and Keep awake tiles follow the new design: a level-filled 1×1 and the apps playing beside the master volume at 4×2; a 2×2 with a ring that empties as a timer runs out.
- Settings and Modules pages redesigned as grouped cards of tidy rows. The microphones' shortcut moved to the Audio devices page.
- The panel no longer nudges itself when a setting other than its look is saved.
- The bundled fonts' licences (Doto, OFL; OCR-A, public domain) are installed with the app.

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

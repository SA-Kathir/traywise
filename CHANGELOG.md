# Changelog

## Unreleased

- **Transparent glass adjusts to what's behind it**: more solid over dark wallpapers, lighter over bright ones. As the panel opens, Traywise measures only the brightness behind it (an 8 × 8 sample, deleted at once).
- Now playing shows a browser's own icon instead of the video's frame, and a music note when there's no picture.
- The picked tab is shown by its icon in the accent colour.
- System monitor: Download and Upload tiles, and a tidier picker for the wide tile.
- The bundled fonts' licences are installed with the app.
- **The Dot look's words are in OCR-A**, a monospaced, machine-readable typeface, lighter than before, beside the dot-matrix numbers.
- **New modules: Wi-Fi and Bluetooth**, with their own Connect tab. Wi-Fi shows the network you're on and lets you join another, typing a new network's password right there. Bluetooth connects and disconnects headphones and speakers. Their tiles go up to 2×2 with the network or device name. New boards start with them; Lock and Desktop icons are in Add tiles.
- **Tiles follow the design** at every size: volume (a level-filled 1×1, a bar with the speaker in it, the apps beside it at 4×2), separate CPU and Memory tiles with clock speed and total memory, a wide tile with any two of CPU, Memory, Download and Upload (picked on the System page), and Keep awake's 2×2 ring with 15 min, 1 hour and ∞ shortcuts.
- **Tile icons show their state**: a moon or a sun for dark mode, a crossed-out eye for hidden desktop icons, a cup or a sleeping moon for Keep awake, and Wi-Fi and Bluetooth off icons.
- **New module: Now playing.** What any app is playing, with cover art, play/pause, skip, a seek bar, shuffle and repeat (when the app allows them). Its tiles come in 1×1 (play/pause) up to 4×2, and the Sound tab lists every app with media, so you can control any of them. Switch it on in Settings → Modules.
- **A new look, Dot, now the default.** Monochrome tiles over a dot grid, numbers in a dot-matrix font, and one accent colour. Switched-on tiles turn into a solid plate with a small accent dot. It follows Windows' light or dark mode. Tiles and cards are frosted glass. Settings → Look has its Accent (Red, Amber, Violet, Blue, Green) and Background (Dots, Plain, or Transparent over the desktop). Liquid and Windows are still there.
- The system monitor's graphs zoom in to what they show, so a quiet CPU's ups and downs are visible, and on its 4×2 tile they're labelled CPU and Network.
- **Add tiles is a tray over the board** in every look, with a size picker: pick 1×1 up to 4×2, then add tiles at that size.
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

# Changelog

## 0.3.0 (2026-11-02)

- **Displays** module: a brightness slider for each screen (the laptop's own, and external monitors that have DDC/CI switched on in their menu), and a **Project** switch like Win+P: PC screen only, Duplicate, Extend or Second screen only. Tiles from 1×1 to 4×2; the page is in the System tab.
- **Power** module: Windows' power mode (Efficiency, Balanced or Performance), the battery with time left, and whether Energy saver is on, with a shortcut to its settings (Windows doesn't let apps switch it). Tiles at 1×1 and 2×1.
- **Per-app output.** In the volume mixer, click the device under an app's name to send it to another output (Spotify on the speakers while a call stays on the headset), or back to Default. Windows remembers it, even after the app or Traywise restarts. An app set to a device that's been unplugged goes back to Default. **Reset all** sends every app back. Apps on other devices now show in the mixer too.
- **Clock** module: the time and date on a tile from 1×1 to 4×2, Digital (dot-matrix numbers) or Analogue (a face of dots). An optional second time zone (picked from a short, searchable list of places) shows on the 4×1, 2×2 and 4×2 tiles, each time labelled with its city: in its own box beside the digital time, or as a second face beside the analogue one. Without one, the bigger tiles give your own time the whole tile. It only updates while the panel is open, once a minute.
- **Focus** module: switches Windows' Do not disturb. The Focus tile turns it on for 25 minutes with a click (or 25 min, 50 min or 1 hour from the 2×2), with a countdown and a ring of dots that empties; it turns itself off when time's up. A plain Do not disturb tile switches it on or off. Both show changes made in Windows too. Windows has no official way to do this, so it uses the same hidden setting Windows' own switch does, and says "Not available" if a Windows update ever removes it.
- **Board pages.** Home can have up to five pages of tiles, each with its own layout:
  - Swipe between pages with a touchpad or touchscreen (or Shift with the mouse wheel); they follow your fingers and snap into place. Ctrl+← / Ctrl+→, PageUp / PageDown and the dots at the bottom work too.
  - In edit mode, pick up a tile and drop it on **Move to** › a page (or a new one) at the bottom.
  - **Pages** in edit mode shows every page as a small outline: go to one, drag to reorder, add, or remove (its tiles go back to Add tiles).
  - A tile is on one page at a time. New installs still start with one page, and your current board becomes page 1.
- **Panel polish.** The title bar, tab bar and Settings' Back button stay pinned while the page scrolls. Switching look or background in Settings no longer closes the panel. Add tiles lists modules first, then asks for the size. Edit mode on a Transparent background shows the look's plain background, and the Add tiles button is no longer covered by tile controls. System Monitor values no longer spill out of small tiles.
- **A new open and close animation.** The title, tabs and tiles come in one after another, each rising out of a soft blur, and they blur away again when it closes (it replaces the edge slide). Every look's panel edge catches the accent along the top and bottom, and brighter where the pointer is; in Dot, hovered tiles get a thin accent ring. A new app icon: four tiles, one switched on.
- **Settings** sections (Modules, Appearance, Opening, System) open and close the same way, with the module switches inside Settings. Dot's light mode is softer: charcoal and off-white instead of black and white.
- **Open from the left or the right edge** (Settings › Opening), and the handle can be in your accent colour.
- **The Liquid look is now called Tint**, since it's cards tinted in one colour and nothing liquid is left. Your saved look carries over, and new installs start with Single hue tiles (the shaded style is still a Settings choice).
- **Fixed:** in Tint with Single hue, hovering a System monitor tile no longer turns it into the shaded look; it stays in its own hue and gets a little lighter.
- **Dot now starts on a plain background** (it used to start over a dot grid) for new installs. Existing installs keep their choice.

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

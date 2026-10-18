# UI notes for milestone 3

Ideas noticed while building milestone 2. Not built; the UI pass decides.

## Shell
- Done (first pass): home view of tiles and full-width rows, each opening the module's own page (`src/App.tsx`, `src/shell/Tile.tsx`).
- With three tiles the grid has a gap; Quick toggles would fill it.
- The window is still 560 px tall; the home view uses about two thirds. Fitting the window to its content would look tidier.

## Keep awake
- Mode picker is two segmented rows (Off / On / Plugged in, then 30 min / 1 h / 2 h). A single on/off switch with a duration menu may read better.
- No sign of keep-awake outside the panel. A tray icon badge or tooltip ("Keeping awake, 42 min left") would help.

## Audio devices
- Output list reuses the segmented control, stacked vertically. Device icons (headphones vs speakers) and shorter names ("Headphones" without "(Realtek(R) Audio)") would scan faster.
- Muted microphones have no sign outside the panel. A tray icon overlay or a small on-screen toast when the shortcut is pressed would confirm it worked.
- The shortcut picker is a plain button ("Press keys…"); it shows raw names like "Ctrl+Alt+M" or "Ctrl+Shift+Space".

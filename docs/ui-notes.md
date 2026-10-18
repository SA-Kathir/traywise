# UI notes for milestone 3

Ideas noticed while building milestone 2. Not built; the UI pass decides.

## Shell
- Modules are simply stacked as cards (`src/App.tsx`). With three or four on, the panel scrolls; a real navigation (tabs, a module grid, or collapsible cards) belongs here.

## Keep awake
- Mode picker is two segmented rows (Off / On / Plugged in, then 30 min / 1 h / 2 h). A single on/off switch with a duration menu may read better.
- No sign of keep-awake outside the panel. A tray icon badge or tooltip ("Keeping awake, 42 min left") would help.

## Audio devices
- Output list reuses the segmented control, stacked vertically. Device icons (headphones vs speakers) and shorter names ("Headphones" without "(Realtek(R) Audio)") would scan faster.
- Muted microphones have no sign outside the panel. A tray icon overlay or a small on-screen toast when the shortcut is pressed would confirm it worked.
- The shortcut picker is a plain button ("Press keys…"); it shows raw names like "Ctrl+Alt+M" or "Ctrl+Shift+Space".

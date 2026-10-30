# Traywise design system (v0.2 direction)

The source of truth is the **Design System** page of the Figma file:
https://www.figma.com/design/grZMZ3PYBcb1Z8PTlibcxn/TrayWise?node-id=11-359

This document lists every token with its value. The app's **Dot** look (`src/shell/dot.css`) is built from them.

## Direction

The look takes its cues from Nothing OS: a monochrome base, one accent, dot-matrix numbers, generous rounded tiles, and a quiet, industrial feel. On top of that sits restrained glass. It's built for Windows 11 and the fixed 360 × 640 tray panel. Traywise has its own identity: no Nothing fonts (NDot/NType), logo or assets.

Decisions made with the owner (2026-10-29):

| Topic | Decision |
|---|---|
| Accent | **Red** by default. Settings offers presets: Red, Amber, Violet, Blue, Green (the `Accent` collection's modes). |
| Dot-matrix type | **Numbers and the wordmark only.** Every word is Segoe UI Variable. |
| Modes | **Dark and light**, following Windows' app mode. |
| Glass | **Two levels.** Tiles get a faint tint and hairline, with no blur. Only floating layers (toast, menu, edit tray) get frost (blur). Fallback if this looks wrong in the app: frost everywhere, measured first. |
| Background | App-drawn and never read from the desktop. **A Setting (Background: Dots / Plain), dot grid by default.** |
| On tiles | **Inverted:** a white plate in dark mode (black in light) with a small accent "live" dot. Red stays rare. |
| Danger | **Orange-red, separate from the accent**, so an error never looks like "on". |
| Red | **#D92B24**, so white text passes WCAG AA (about 4.9:1). |
| Words (2026-10-30, in the app) | **OCR-A** (public domain) instead of Segoe UI Variable, in one weight and a size smaller, so the words set off the dot-matrix numbers. The type ramp below is the Figma file's. |

## Tokens

CSS names use a `--tw-` prefix and replace `/` with `-` (Figma variable code syntax is set to these names). Colours below are hex. A `/NN` suffix means NN% alpha.

### Primitives (hidden from Figma pickers; reference only)

```
gray/0 #FFFFFF   gray/50 #F4F4F2   gray/100 #EAEAE7  gray/150 #DCDCD8  gray/200 #C9C9C5
gray/300 #A9A9A5 gray/400 #8A8A87  gray/500 #6C6C69  gray/600 #51514F  gray/700 #3A3A39
gray/800 #262627 gray/850 #1C1C1E  gray/900 #141416  gray/950 #0B0B0C  gray/1000 #000000
white/N and black/N for N in 4 6 8 10 12 14 20 24 40 60 72 88 (alpha %)
glass/float-dark #1C1C1E/72        glass/float-light #FAFAF8/74
red    300 #FF7A72  500 #D92B24  700 #B81F19
orange 500 #FF7A1A  700 #B84A08
amber  300 #FFC774  500 #F2A33A  700 #A8620A
violet 300 #C7B4FF  500 #A98BFF  700 #6A48DB
blue   300 #9CCBFF  500 #5AA9FF  700 #1D64C8
green  300 #7FE0AA  500 #34C77B  700 #137F4B
ink/on-light #140F0A
```

### Accent (modes = Settings presets)

| Token | Red (default) | Amber | Violet | Blue | Green |
|---|---|---|---|---|---|
| `--tw-accent-base` | #D92B24 | #F2A33A | #A98BFF | #5AA9FF | #34C77B |
| `--tw-accent-strong` | #B81F19 | #A8620A | #6A48DB | #1D64C8 | #137F4B |
| `--tw-accent-soft` | #FF7A72 | #FFC774 | #C7B4FF | #9CCBFF | #7FE0AA |
| `--tw-accent-on` (text on accent) | #FFFFFF | #140F0A | #140F0A | #140F0A | #140F0A |
| `--tw-accent-tint` | #D92B24/16 | #F2A33A/16 | #A98BFF/16 | #5AA9FF/16 | #34C77B/16 |

### Theme (semantic; components only use these)

Accent rows show the Red preset.

| Token | Dark | Light | Use |
|---|---|---|---|
| `--tw-bg-base` | #0B0B0C | #F4F4F2 | Panel background |
| `--tw-bg-deep` | #000000 | #EAEAE7 | Bottom of the background gradient |
| `--tw-bg-dot` | #FFFFFF/10 | #000000/12 | Dot-grid dots |
| `--tw-glass-tile` | #FFFFFF/6 | #FFFFFF/72 | Level-1 glass: tiles, cards, controls |
| `--tw-glass-tile-hover` | #FFFFFF/10 | #FFFFFF/88 | |
| `--tw-glass-tile-pressed` | #FFFFFF/4 | #FFFFFF/60 | |
| `--tw-glass-float` | #1C1C1E/72 | #FAFAF8/74 | Level-2 glass (with blur): toast, menu, tray |
| `--tw-glass-edge` | #FFFFFF/8 | #000000/6 | 1 px inside edge |
| `--tw-glass-edge-strong` | #FFFFFF/14 | #000000/12 | Floating edges, hover, edit-mode dashes |
| `--tw-text-primary` | #F4F4F2 | #0B0B0C | |
| `--tw-text-secondary` | #FFFFFF/60 | #000000/60 | |
| `--tw-text-tertiary` | #FFFFFF/40 | #000000/40 | |
| `--tw-text-disabled` | #FFFFFF/24 | #000000/24 | |
| `--tw-text-inverse` | #0B0B0C | #FFFFFF | Text on `control-fill` (selected pills) |
| `--tw-icon-primary` | #F4F4F2 | #0B0B0C | |
| `--tw-icon-secondary` | #FFFFFF/60 | #000000/60 | |
| `--tw-control-track` | #FFFFFF/12 | #000000/10 | Switch-off, slider and segmented tracks |
| `--tw-control-fill` | #F4F4F2 | #0B0B0C | Slider fill, selected segment, play button |
| `--tw-control-thumb` | #FFFFFF | #FFFFFF | Slider thumb |
| `--tw-control-quiet-hover` | #FFFFFF/8 | #000000/6 | Quiet button and list-row hover |
| `--tw-accent-base` (theme) | accent-base | accent-base | |
| `--tw-accent-text` | accent-base | accent-strong | Accent used as text |
| `--tw-accent-on` (theme) | accent-on | accent-on | |
| `--tw-accent-soft-bg` | accent-tint | accent-tint | Drop target, tinted backgrounds |
| `--tw-tile-on-fill` | control-fill (#F4F4F2) | control-fill (#0B0B0C) | Fill of "on" tiles (inverted) |
| `--tw-tile-on-text` | text-inverse (#0B0B0C) | text-inverse (#FFFFFF) | Text and icons on "on" tiles |
| `--tw-status-success` | #34C77B | #137F4B | |
| `--tw-status-warning` | #F2A33A | #A8620A | |
| `--tw-status-danger` | #FF7A1A | #B84A08 | Orange-red, separate from the accent |
| `--tw-effect-shadow` | #000000/40 | #000000/14 | Shadow colour (floating layers only) |
| `--tw-effect-scrim` | #000000/40 | #000000/20 | Behind the edit tray |

### Dimension

| Token | Value | Notes |
|---|---|---|
| `--tw-space-0 … 12` | 0, 4, 8, 12, 16, 20, 24, 32 (`space-8`), 40 (`space-10`), 48 (`space-12`) | 4 px base. Name = multiple of 4. |
| `--tw-radius-xs` | 4 | |
| `--tw-radius-sm` | 8 | Panel corners are Windows' own 8 px. |
| `--tw-radius-md` | 12 | List rows, tile slider bar, artwork |
| `--tw-radius-lg` | 16 | Toast, floating cards |
| `--tw-radius-tile` | 20 | Every board tile and the edit tray |
| `--tw-radius-full` | 999 | Buttons, switches, segmented, chips |
| `--tw-blur-none` | 0 | |
| `--tw-blur-float` | 24 in Figma | Equals CSS `blur(12px)`. |
| `--tw-board-columns` | 4 | |
| `--tw-board-cell` | 76 | Square cells |
| `--tw-board-gutter` | 8 | |
| `--tw-board-inset` | 16 | Panel side padding (the code uses padding + scrollbar gutter to reach 328) |
| `--tw-panel-width` × `--tw-panel-height` | 360 × 640 | Fixed |
| `--tw-stroke-hairline` | 1 | All glass edges, inside |
| `--tw-icon-size` | 20 | On a 24 px grid |
| `--tw-icon-stroke` | 1.5 | |

### Motion

| Token | Value | Use |
|---|---|---|
| `--tw-duration-instant` | 80 ms | Hover and press feedback |
| `--tw-duration-fast` | 140 ms | Switch thumb, tab change, tile on/off |
| `--tw-duration-base` | 220 ms | Toast in, tile resize, content swaps |
| `--tw-duration-slow` | 320 ms | Edit mode enter/exit, tray slide |
| `--tw-duration-edit-wiggle` | 900 ms | Optional idle wiggle in edit mode (±0.6°) |
| `--tw-ease-standard` | cubic-bezier(0, 0, 0, 1) | Default (Windows 11 fast-out) |
| `--tw-ease-exit` | cubic-bezier(0.4, 0, 1, 1) | Things leaving |
| `--tw-ease-snap` | cubic-bezier(0.34, 1.4, 0.64, 1) | Only for a tile dropping or resizing into place |
| `--tw-ease-linear` | linear | Progress, timers |

Under `prefers-reduced-motion`, all durations are 0 and there's no wiggle.

### Elevation (Figma effect styles)

| Style | Value |
|---|---|
| Elevation/0 Flat | none. Tiles at rest never cast a shadow. |
| Elevation/1 Lift | 0 2 8 `effect-shadow`. A tile being dragged. |
| Elevation/2 Float | 0 8 24 `effect-shadow`. Toast, menu, tray. |
| Glass/Level 2 Frost | background blur 12 px (CSS) + Elevation/2 |

## Typography

| Style | Font | Size / line | Tracking | Use |
|---|---|---|---|---|
| Display/XL | Doto Black | 64 / 60 | -1% | Hero numbers, 2×2 and 4×2 |
| Display/L | Doto Black | 44 / 42 | -1% | Large readouts |
| Display/M | Doto Black | 28 / 28 | 0 | Numbers on 1×1 and 2×1 tiles |
| Display/S | Doto Black | 20 / 20 | 0 | Small numbers, times, size picker |
| Display/Wordmark | Doto Black | 18 / 18 | 8%, uppercase | TRAYWISE only |
| UI/Title | Segoe UI Variable Display Semibold | 20 / 26 | 0 | Page titles |
| UI/Subtitle | Segoe UI Variable Text Semibold | 15 / 20 | 0 | Section heads, media title |
| UI/Body strong | Segoe UI Variable Text Semibold | 13 / 18 | 0 | Tile titles, buttons, list titles |
| UI/Body | Segoe UI Variable Text Regular | 13 / 18 | 0 | Default (the app's 13 px) |
| UI/Caption strong | Segoe UI Variable Text Semibold | 12 / 16 | 0 | 1×1 tile labels |
| UI/Caption | Segoe UI Variable Text Regular | 12 / 16 | 0 | Secondary lines, values |
| UI/Overline | Segoe UI Variable Text Semibold | 11 / 14 | 6%, uppercase | Tile labels (CPU, VOLUME, NOW PLAYING) |

- Doto is OFL-licensed and on Google Fonts. Bundle the woff2 and don't load it from a CDN, which keeps the app offline and fast. Only the Black weight is used.
- Use dot type only for digits, times and the wordmark. Units sit beside the number in Segoe (`23` then `%`).
- Live Segoe numbers use `font-variant-numeric: tabular-nums`. Doto is monospaced.
- Minimum text size is 11 px.
- In Figma, the UI styles use **Inter as a stand-in**, because Segoe UI Variable isn't available to Figma on this machine. They're bound to the `font/ui` variable: set it to "Segoe UI Variable" once Figma can see the font, and every style follows.

## Glass recipe

```css
/* Level 1: tiles, cards, controls. No blur. */
.tile  { background: var(--tw-glass-tile); box-shadow: inset 0 0 0 1px var(--tw-glass-edge); border-radius: 20px; }
.tile:hover { background: var(--tw-glass-tile-hover); }

/* Level 2: only things floating over the board. */
.float { background: var(--tw-glass-float); backdrop-filter: blur(12px) saturate(1.2);
         box-shadow: inset 0 0 0 1px var(--tw-glass-edge-strong), 0 8px 24px var(--tw-effect-shadow);
         border-radius: 16px; }

/* App-drawn backgrounds (pick one in Settings, or keep one after comparing) */
.bg-dots  { background: radial-gradient(var(--tw-bg-dot) 0.9px, transparent 1.1px) 0 0 / 12px 12px,
                        linear-gradient(var(--tw-bg-base), var(--tw-bg-deep)); }
.bg-plain { background: linear-gradient(var(--tw-bg-base) 0%, var(--tw-bg-base) 30%, var(--tw-bg-deep) 100%); }
```

The layers are background → tint → 1 px edge → (level 2 only) blur → (level 2 only) shadow. There are no white specular highlights, no blobs and no refraction. Level-2 blur only appears while something floats, so it costs nothing at rest.

## Iconography

- 24 px grid with a 20 px live area (2 px padding). Keylines: a 20 px circle and an 18 px square with 2.5 px corner radius.
- 1.5 px stroke, round caps and joins. Strokes sit on half-pixels so they stay crisp at 100, 125 and 150% scaling.
- **Signature:** small details become solid dots, echoing the dot-matrix type (the Wi-Fi origin, coffee steam, the CPU core, the drag grip).
- One colour per icon (`icon-primary` or `icon-secondary`). Accent only for an active state. Sizes: 20 px in tiles, rows and buttons; 16 px in chips.
- The set in Figma (`Icon/…`): volume, volume-off, wifi, bluetooth, cpu, memory, coffee, headphones, play, pause, next, moon, sliders, edit, plus, minus, check, grip, resize, chevron.

## Board grid

These numbers match `src/shell/liquid.css` (`.board`):

- 4 columns, 76 px square cells, 8 px gaps, 328 px wide.
- Sizes: 1×1 76×76 · 2×1 160×76 · **4×1 328×76** · 2×2 160×160 · 4×2 328×160.
- Width = cols × 76 + (cols − 1) × 8, and the same for height.
- Header 48 px, board starting at y 64; six rows are visible before scrolling.
- **One difference:** the code's `--tile-pad` is `10px 12px`; the design uses 12 px all round. Either works, so it's the code's call.

## Components (Figma)

| Component | Variants | Spec |
|---|---|---|
| Background | Pattern = Dots / Plain | 360×640. Dot grid: 12 px pitch, 1.8 px dots, `bg-dot`. Plain: `bg-base` fading to `bg-deep`. |
| Switch | Checked × State (Rest/Hover/Disabled) | 40×20 pill. Off: track + hairline, grey 12 px knob. On: accent track, `accent-on` knob. Knob is 14 px on hover. |
| Slider | Style (Inline/Tile) × State | Inline: 4 px track, neutral fill, 16 px white thumb. Tile: a 28–36 px bar where the fill is the thumb. The fill is neutral, never accent. |
| Button | Style (Primary/Secondary/Quiet) × State (Rest/Hover/Pressed/Disabled) | Pill, 32 px tall, 16 px side padding, optional 16 px icon. One Primary per view. Hover +12% white, pressed +12% black. Disabled is 40% opacity. |
| Icon button | Style (Quiet/Glass) × State | 32 px circle, 20 px icon. |
| Segmented control | Selected = 1/2/3 | Track pill, 3 px inset. The selected segment is `control-fill` with `text-inverse` (inverted pill). |
| Tabs | Selected = 1/2/3 | Text tabs. Selected: primary, semibold, with a 4 px accent dot below. |
| List row | Trailing (None/Switch/Chevron/Value) × State | 328 wide, 8/12 padding, radius 12. Hover wash, no dividers. |
| Panel header | Mode = Normal/Edit | 48 px. Normal: TRAYWISE wordmark + accent dot, then the Edit and Settings icon buttons. Edit: "Edit board" + hint + Primary "Done". |
| Toast | Tone = Neutral/Success/Danger | Level-2 glass, 328×44, 8 px status dot, optional quiet action. Sits 16 px above the bottom; auto-hides after 4 s. |
| Edit chrome | Part (Remove badge/Resize handle/Drag grip) × State | Remove: 24 px `control-fill` with a minus, red on hover. Resize: a 2.5 px corner arc, accent while dragging. Grip: 3×2 dots on a frosted pill. |
| Edit overlay | Size (1×1/2×1/4×1/2×2/4×2) × State (Idle/Selected) | Idle: dashed `glass-edge-strong`. Selected: 2 px solid accent; the tile lifts (Elevation/1), scales to 102% and tilts −2°. |
| Drop slot | Size (1×1/2×1) × State (Empty/Target) | Dashed outline with a plus. Target: accent tint and accent dashes. |
| Size picker | Selected = 1×1/2×1/4×1/2×2/4×2 | Segmented, with sizes in Doto. Hide sizes the tile doesn't support. Sits on its own row in the tray. |
| Edit tray | none | Level-2 sheet, 344 wide: handle, "Add tiles" + size picker, then rows with + buttons. |

### Tiles

Every tile is level-1 glass with a 20 px radius and 12 px padding. An "on" tile is inverted (`tile-on-fill`, `tile-on-text`, secondary text at 72%) and shows a 6 px `accent-base` live dot 12 px from its top-right corner. The 2×2 toggle shows its accent switch instead of the dot, and the 2×2 keep-awake tile its accent dot ring.

| Tile | Sizes in Figma | Content |
|---|---|---|
| Tile/Toggle | 1×1, 2×1, 2×2 × Off/On | Icon (instance swap), title, state line. 2×2 adds a switch. |
| Tile/Slider (volume) | 1×1, 2×1, 4×1, 2×2, 4×2 | 1×1: the whole tile is a vertical level. 2×1/4×1/2×2: a bar where the fill is the thumb, plus a Doto value. 4×2: the per-app mixer. |
| Tile/Stat (CPU/RAM) | 1×1, 2×1, 4×1, 2×2, 4×2 | Doto number + unit, and a dot-matrix history graph (4 px dots, 3 px gap, newest column in accent). 4×1 and 4×2 show CPU and RAM side by side. |
| Tile/Media | 2×1, 2×2, 4×2 | Artwork (dot-pattern placeholder), title, artist, accent progress bar, play/pause and next, times in Doto. |
| Tile/Keep awake | 1×1, 2×1, 2×2 × Off/On | Countdown in Doto. 2×2 has a 24-dot ring that empties as time runs out, plus quick durations. |

## Screens (Figma, "10 Screens")

1. Home board, normal (dark, dot grid, with a toast).
2. Home board, edit mode (dark). One tile is mid-drag toward its target slot, and one slot is empty.
3. Edit mode with the add-tile tray open (light, plain background).
4. Normal, plain background (dark).
5. Normal, light mode.
6. Normal, Amber accent preset.

## Decided after review (2026-10-29)

The owner chose these from side-by-side mockups: inverted on-tiles with an accent live dot; an orange-red danger colour; red #D92B24; Background as a Setting (dots by default); 4×1 in the size picker. Two-level glass stays until it's seen in the real app.

## Open questions

1. **Two-level glass in the real app.** If it doesn't look right, the fallback is frost on tiles too, which needs a CPU measurement first.
2. **Segoe UI Variable in Figma.** Install it for Figma (the desktop app or font helper), then set the `font/ui` variable.
3. **Tile padding.** The code uses `10px 12px`, the design 12 px all round. Either works.

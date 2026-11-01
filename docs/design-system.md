# Traywise design system (0.2.0 as shipped, 0.3.0 designs)

The source of truth is the **Design System** page of the maintainers' Figma file. It has three pages:

- **Design System**: foundations and components.
- **v0.2 · Shipped**: what 0.2.0 looks like (home, tabs, Connect, Settings, Modules, edge opening).
- **v0.3 · Next release**: clock, focus, board pages (as built), displays, power, project, per-app output, and the other looks.

This document lists every token with its value. The app's **Dot** look (`src/shell/dot.css`) is built from them. Liquid and Windows keep their own styles (`src/shell/liquid.css`); see [Other looks](#other-looks).

## Direction

The look takes its cues from Nothing OS: a monochrome base, one accent, dot-matrix numbers, generous rounded tiles, and a quiet, industrial feel, under frosted glass. It's built for Windows 11 and the fixed 360 × 640 tray panel. Traywise has its own identity: no Nothing fonts (NDot/NType), logo or assets.

Decisions with the owner:

| Topic | Decision |
|---|---|
| Accent | **Red (#D92B24)** by default, chosen so white text passes WCAG AA (about 4.9:1). Settings › Accent: Red, Amber, Violet, Blue, Green. |
| Numbers | **Doto** (OFL, bundled, Black only) for numbers, times and the TRAYWISE wordmark. Nothing else. |
| Words | **OCR-A** (public domain, bundled, one weight, no faux bold), a size smaller than the Segoe ramp because it's wide. |
| Modes | **Dark and light**, following Windows' app mode. |
| Glass | **Every tile and card is frosted** (level-2 glass), the owner's pick after seeing it in the app. |
| Background | **Dots** (default), **Plain** or **Transparent** (Settings › Background). It's never read from the desktop, with one exception: Transparent samples an 8 × 8 brightness once as the panel opens, then deletes it. |
| On tiles | **Inverted plate** (white in dark, black in light) with a 6 px accent "live" dot. Red stays rare. |
| Danger | **Orange-red, separate from the accent**, so an error never looks like "on". |
| Board pages (0.3) | As built: **plain dots** at the panel bottom, a **page bar** in edit mode, and tiles move by **dropping them on a page's dot**. (The owner kept their own picks over the Figma pill and Move to bar.) |
| Clock (0.3) | **Digital or Analogue** (a setting). The second time zone always sits in its own labelled well. |

## Tokens

CSS names use a `--tw-` prefix and replace `/` with `-` (Figma variable code syntax is set to these names). Colours below are hex. A `/NN` suffix means NN% alpha.

### Primitives (hidden from Figma pickers; reference only)

```
gray/0 #FFFFFF   gray/50 #F4F4F2   gray/100 #EAEAE7  gray/150 #DCDCD8  gray/200 #C9C9C5
gray/300 #A9A9A5 gray/400 #8A8A87  gray/500 #6C6C69  gray/600 #51514F  gray/700 #3A3A39
gray/800 #262627 gray/850 #1C1C1E  gray/900 #141416  gray/950 #0B0B0C  gray/1000 #000000
white/N and black/N for N in 4 6 8 10 12 14 20 24 40 60 72 88 (alpha %)
glass/float-dark #1C1C1E/72        glass/float-light #FAFAF8/74
glass/float-hover-dark #303032/76  glass/float-hover-light #FFFFFF/90
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
| `--tw-glass-tile` | #FFFFFF/6 | #FFFFFF/72 | Unfrosted fills: wells, chips, glass icon buttons |
| `--tw-glass-tile-hover` | #FFFFFF/10 | #FFFFFF/88 | |
| `--tw-glass-tile-pressed` | #FFFFFF/4 | #FFFFFF/60 | |
| `--tw-glass-float` | #1C1C1E/72 | #FAFAF8/74 | **Every tile and card** (with blur), toast, tray |
| `--tw-glass-float-hover` | #303032/76 | #FFFFFF/90 | Hovered tile or card |
| `--tw-glass-edge` | #FFFFFF/8 | #000000/6 | Hairlines: row dividers, wells |
| `--tw-glass-edge-strong` | #FFFFFF/14 | #000000/12 | The 1 px edge of tiles and cards, edit-mode dashes |
| `--tw-text-primary` | #F4F4F2 | #0B0B0C | |
| `--tw-text-secondary` | #FFFFFF/60 | #000000/60 | |
| `--tw-text-tertiary` | #FFFFFF/40 | #000000/40 | |
| `--tw-text-disabled` | #FFFFFF/24 | #000000/24 | |
| `--tw-text-inverse` | #0B0B0C | #FFFFFF | Text on `control-fill` (selected pills, on-tiles) |
| `--tw-icon-primary` | #F4F4F2 | #0B0B0C | |
| `--tw-icon-secondary` | #FFFFFF/60 | #000000/60 | |
| `--tw-control-track` | #FFFFFF/12 | #000000/10 | Switch-off, slider and segmented tracks, wells, text fields |
| `--tw-control-fill` | #F4F4F2 | #0B0B0C | Slider fill, selected segment, play button |
| `--tw-control-thumb` | #FFFFFF | #FFFFFF | Slider thumb |
| `--tw-control-quiet-hover` | #FFFFFF/8 | #000000/6 | Quiet button, row and menu-item hover |
| `--tw-accent-text` | accent-base | accent-strong | Accent used as text |
| `--tw-accent-soft-bg` | accent-tint | accent-tint | Drop target, TOMORROW tag |
| `--tw-tile-on-fill` | control-fill (#F4F4F2) | control-fill (#0B0B0C) | Fill of "on" tiles (inverted) |
| `--tw-tile-on-text` | text-inverse (#0B0B0C) | text-inverse (#FFFFFF) | Text and icons on "on" tiles |
| `--tw-status-success` | #34C77B | #137F4B | |
| `--tw-status-warning` | #F2A33A | #A8620A | Battery below 20 % |
| `--tw-status-danger` | #FF7A1A | #B84A08 | Errors, battery below 10 % |
| `--tw-effect-shadow` | #000000/40 | #000000/14 | Shadow colour |
| `--tw-effect-scrim` | #000000/40 | #000000/20 | Behind the Add tiles tray |

**Transparent background** (`.look-dot.clear`): the glass alpha follows the brightness behind the panel, `0.92 − brightness × 0.24` (brightness 0–1 from the 8 × 8 sample), for both `glass-float` and `glass-float-hover`. The header and tab bar get their own frosted cards.

### Dimension

| Token | Value | Notes |
|---|---|---|
| `--tw-space-0 … 12` | 0, 4, 8, 12, 16, 20, 24, 32 (`space-8`), 40 (`space-10`), 48 (`space-12`) | 4 px base. Name = multiple of 4. |
| `--tw-radius-xs` | 4 | |
| `--tw-radius-sm` | 8 | Panel corners are Windows' own 8 px; menu items. |
| `--tw-radius-md` | 12 | List rows, text fields, wells, artwork |
| `--tw-radius-lg` | 16 | Toast |
| `--tw-radius-tile` | 20 | Every tile, card and the tray |
| `--tw-radius-full` | 999 | Buttons, switches, segmented, chips |
| `--tw-blur-float` | 12px | CSS `blur(12px) saturate(1.2)` (24 in Figma) |
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
| `--tw-duration-base` | 220 ms | Toast in, tile resize, page slide (0.3) |
| `--tw-duration-slow` | 320 ms | Edit mode enter/exit, tray slide |
| `--tw-ease-standard` | cubic-bezier(0, 0, 0, 1) | Default (Windows 11 fast-out) |
| `--tw-ease-exit` | cubic-bezier(0.4, 0, 1, 1) | Things leaving |
| `--tw-ease-snap` | cubic-bezier(0.34, 1.4, 0.64, 1) | Only for a tile dropping or resizing into place |
| `--tw-ease-linear` | linear | Progress, timers |

Under `prefers-reduced-motion`, all durations are 0.

### Elevation (Figma effect styles)

| Style | Value |
|---|---|
| Elevation/1 Lift | 0 2 8 `effect-shadow`. A tile being dragged. |
| Elevation/2 Float | 0 8 24 `effect-shadow`. On-tiles (they have no blur). |
| Glass/Level 2 Frost | background blur 12 px (CSS) + Elevation/2. Every tile, card and floating layer. |

## Typography

The look sets 11 px as its base size, because OCR-A is wide (about 0.7 em per letter).

| Style | Font | Size / line | Tracking | Use |
|---|---|---|---|---|
| Display/XL | Doto Black | 56 / 56 | 0 | Hero reading on 2×2 and 4×2 stats, 4×2 clock |
| Display/L | Doto Black | 40 / 40 | 0 | Big volume, brightness and 2×1/2×2 clock |
| Display/M | Doto Black | 28 / 28 | 0 | Default reading (`.num`), countdowns |
| Display/S | Doto Black | 20 / 20 | 0 | 1×1 readings, size picker, times |
| Display/XS | Doto Black | 16 / 16 | 0 | 1×1 countdowns, second-zone time, small values |
| Display/Wordmark | Doto Black | 18 / 18 | 8%, uppercase | TRAYWISE only |
| UI/Title | OCR-A | 20 / 26 | 0 | Page titles (Settings, Modules, Edit board) |
| UI/Subtitle | OCR-A | 15 / 20 | 0 | Tray and sheet titles |
| UI/Body strong | OCR-A | 13 / 18 | 0 | Media title |
| UI/Caption strong | OCR-A | 12 / 16 | 0 | Tile labels and states, row labels |
| UI/Body | OCR-A | 11 / 16 | 0 | Default text, buttons |
| UI/Overline | OCR-A | 11 / 14 | 6%, uppercase | Section heads and tile overlines (CPU, VOLUME) |
| UI/Caption | OCR-A | 10 / 14 | 0 | Hints, idle costs, secondary lines |

- Use dot type only for digits, times and the wordmark. A unit sits beside its number in OCR-A (`23` then `%`), on the same baseline.
- Minimum text size is 10 px.
- In Figma, OCR-A isn't available, so the UI styles use **Share Tech Mono as a stand-in**, bound to the `font/ui` variable. Install OCR-A for Figma (the font is in `src/assets/fonts`), set `font/ui` to it, and every style follows.

## Glass recipe

```css
/* Every tile and card */
.tile, section { background: var(--tw-glass-float); backdrop-filter: blur(var(--tw-blur-float)) saturate(1.2);
                 box-shadow: inset 0 0 0 1px var(--tw-glass-edge-strong), 0 8px 24px var(--tw-effect-shadow);
                 border-radius: var(--tw-radius-tile); }
.tile:hover    { background: var(--tw-glass-float-hover); }
/* Switched on: the inverted plate (no blur) and a live dot */
.tile.on       { background: var(--tw-tile-on-fill); color: var(--tw-tile-on-text); }
.tile.on::after { /* 6 px accent dot, 12 px from the top right */ }
/* App-drawn backgrounds */
.bg-dots  { background: radial-gradient(var(--tw-bg-dot) 0.9px, transparent 1.1px) 0 0 / 12px 12px,
                        linear-gradient(var(--tw-bg-base), var(--tw-bg-deep)); }
.bg-plain { background: linear-gradient(var(--tw-bg-base) 30%, var(--tw-bg-deep)); }
```

There are no white specular highlights, no blobs and no refraction. The background never moves, so the blur is computed once.

## Iconography

- 24 px grid with a 20 px live area (2 px padding). Keylines: a 20 px circle and an 18 px square with 2.5 px corner radius.
- 1.5 px stroke, round caps and joins. Strokes sit on half-pixels so they stay crisp at 100, 125 and 150% scaling.
- **Signature:** small details become solid dots (the Wi-Fi origin, coffee steam, the CPU core, the battery cells, the drag grip). A screen that is off (Project) is drawn as four corner dots.
- One colour per icon (`icon-primary` or `icon-secondary`). Accent only for an active state (the selected tab, a connected network or device).
- The app draws icons with Segoe Fluent glyphs. The Figma set (`Icon/…`) shows the intended style and is what new glyphs should match: volume, volume-off, wifi, bluetooth, cpu, memory, coffee, headphones, play, pause, next, previous, shuffle, repeat, moon, night, dark-mode, sliders, settings, power, home, laptop, tools, back, edit, plus, minus, check, grip, resize, chevron, mic, music, speaker, sun, display, battery, charging, gauge, leaf, bolt, focus, bell-off, globe, clock, pages, more, arrow-right, trash, lock, download, upload, eye-off, project-pc, project-duplicate, project-extend, project-second.

## Panel layout

- Header 48 px at y 8 (Normal: wordmark + accent dot, then Edit, Settings, Quit; Edit: "Edit board" + hint + Done; Page: Back + title).
- Icon tab bar, 36 px, at y 58: Home, Sound, System, Connect, Tools (only categories with a module on). The selected icon is in the accent colour; there's no track.
- The board starts at y 100. In edit mode, the edit bar (Auto-arrange, Reset, Add tiles) replaces the tabs, and from 0.3 the page bar sits under it.
- Board grid (matches `.board`): 4 columns, 76 px square cells, 8 px gaps, 328 px wide. Sizes 1×1 76×76, 2×1 160×76, 4×1 328×76, 2×2 160×160, 4×2 328×160. Dot pads tiles 12 px all round.

## Components (Figma, Design System and v0.2 pages)

| Component | Variants | Spec |
|---|---|---|
| Background | Pattern = Dots / Plain | 360×640. Dots: 12 px pitch, 1.8 px dots, `bg-dot`. Plain: `bg-base` fading to `bg-deep`. |
| Switch | Checked × State | 40×20 pill. Off: track + hairline, grey 12 px knob. On: accent track, `accent-on` knob. 14 px knob on hover. |
| Slider | Style (Inline/Tile) × State | Inline: 4 px track, neutral fill, 16 px white thumb. Tile: a 28–36 px bar whose fill is the thumb. Never accent. |
| Button | Style (Primary/Secondary/Quiet) × State | Pill, 32 px tall, 16 px side padding. One Primary per view. |
| Icon button | Style (Quiet/Glass) × State | 32 px circle, 20 px icon. |
| Segmented control | Selected | Track pill, 3 px inset; the selected segment is the inverted pill. |
| Tab bar | Selected = Home/Sound/System/Connect/Tools | Icon-only; selected icon in accent. |
| Panel header | Mode = Normal/Edit/Page | See [Panel layout](#panel-layout). |
| Edit bar | Reset = Rest/Confirm reset | Auto-arrange switch, Reset (second click: "Reset board?" on the inverted pill), Add tiles. |
| Setting row | Type = Switch, Switch + desc, Value, Link, Stacked choice, Colour | Uniform rows inside a card, 1 px `glass-edge` divider between rows. Colour: 22 px swatches, the picked one ringed in text-primary. |
| Module row | On = Yes/No | 36 px icon well, name, what it does, "Idle: …" in text-tertiary, switch. |
| Network row | State = Connected/Saved/Open/Joining | Wi-Fi icon (accent when connected), name, state, lock if secured. Joining expands in place: password field + Cancel / Join. |
| Device row | State = Connected/Paired/Connecting | Device icon (accent when connected), name, state, Connect / Disconnect pill. |
| Text input | State = Rest/Focus | `control-track` fill, radius 12, no border; focus = 1.5 px accent ring. |
| Toast | Tone = Neutral/Success/Danger | Frosted, 328×44, 8 px status dot, optional quiet action. |
| Edit chrome, Edit overlay, Drop slot, Size picker, Edit tray | as before | Remove badge top left, grip top centre, resize corner bottom right; dashed idle edge, 2 px accent when selected. The size picker has five sizes. |
| **Page dots** (0.3) | Count × Active × Drop | Plain dots centred at the panel bottom, sticky under the board. 6 px dots in 16 px hit targets: current page full ink, others 30 %, hover 60 %. A drop target grows to 12 px in the accent. Hidden with one page. |
| **Page bar** (0.3) | State = Rest/Confirm remove | Under the edit bar: ‹ (move page left), "Page 1 of 3" (numbers in Doto), › (move page right), trash, +. |

### Tiles

Every tile is frosted, with a 20 px radius and 12 px padding. An "on" tile is the inverted plate with a 6 px accent live dot, 12 px from its top-right corner (2×2 toggles show their switch instead, timers their ring). Wider tiles put the icon in a 40 px round well at the left.

| Tile | Sizes | Content |
|---|---|---|
| Toggle (Wi-Fi, Bluetooth, dark mode, desktop icons, lock, mics, output, Do not disturb) | 1×1, 2×1, 2×2 | Icon, name, state ("On", network or device name). 2×2 adds a switch. |
| Volume | 1×1, 2×1, 4×1, 2×2, 4×2 | 1×1: the whole tile is the level. Bar whose fill is the thumb, value in Doto; 4×2 adds the apps playing. |
| Stat (CPU, Memory, Download, Upload; Two readings) | 1×1, 2×1, 2×2; 4×1, 4×2 | Doto number + unit, dot-matrix history (lit dots up to each reading, newest column in accent). |
| Now playing | 1×1, 2×1, 2×2, 4×2 | Cover art (or a music note on the track colour), title, artist, accent progress line, play/pause (inverted circle), next; times in Doto. |
| Keep awake | 1×1, 2×1, 2×2 | Countdown in Doto; 2×2 has a 24-dot ring (lit in accent) and quick durations. |
| **Clock** (0.3) | 1×1, 2×1, 4×1, 2×2, 4×2 × Digital/Analogue | See [Clock](#clock-and-focus). |
| **Focus** (0.3) | 1×1, 2×1, 2×2 | Keep-awake anatomy: countdown, 24-dot ring at 2×2, "Do not disturb on", quick durations 25m · 50m · 1h. |
| **Brightness** (0.3) | 1×1, 2×1, 4×1, 4×2 (2×2 designed) | Volume anatomy with a sun; 4×2 has one row per display. |
| **Power mode** (0.3) | 1×1, 2×1 | Leaf / gauge / bolt. 1×1 cycles; 2×1 is a three-step selector. No accent. |
| **Battery** (0.3) | 1×1, 2×1 | Dot-matrix cell gauge, % in Doto, time left; status colours below 20 / 10 %. |
| **Project** (0.3) | 1×1, 2×1 | Win+P: PC only / Duplicate / Extend / Second only. 1×1 cycles; 2×1 four-step selector; "One screen" dimmed when only one is connected. |

## 0.3.0 designs

### Board pages

As built on `feature/board-pages` (the owner kept their own picks over the earlier Figma direction). Figma: v0.3 › Board pages.

- Up to 5 boards side by side. Turn pages with Ctrl+←/→, PageUp/PageDown, a sideways wheel or Shift+wheel, a drag, or a dot.
- Page dots: plain 6 px dots (16 px hit targets), centred at the panel bottom and sticky under the board. The current page is full ink, the others 30 % (60 % on hover). Hidden with one page. In edit mode they sit above the edit area.
- Edit mode: a page bar under the edit bar, ‹ "Page N of M" › trash +. ‹ and › move this page left or right; + adds a page after this one; trash removes it. Removing a page with tiles asks first, and its tiles go back to Add tiles. Suggested confirm pattern: the trash turns into "Remove page?" on the inverted pill, like "Reset board?".
- Moving a tile to another page: drag it onto that page's dot. The dot grows to 12 px in the accent while the tile is over it; drop it there.
- A new page shows empty drop slots, "This page is empty" and a primary "Add tiles".

### Clock and focus

Figma: v0.3 › Clock + Focus.

- Settings: Clock style, Digital or Analogue.
- Digital: Doto digits. 1×1 stacks hours over minutes with the day below; 2×1 has time + date overline; 4×2 has "HERE · SAT 31 OCT", a 56 px time and the city.
- Analogue: a dot-matrix face (60 tick dots, the 5-minute ones brighter), hands drawn as dots, the minute hand's tip in the accent, and digits beside it from 2×1.
- **Second time zone** (from 4×1 / 2×2): always in its own inset well (`control-track`, radius 12) with a globe, the CITY in overline caps, the day difference in accent text (+1) and the time in Doto. At 4×2 the well is a full column with "SUN 1 · +8 H", a day/night glyph and a TOMORROW / YESTERDAY tag (`accent-soft-bg`). The other zone's time is never shown bare next to local time.
- One timer, on the minute.
- Focus starts a Windows focus session, which also silences notifications. Do not disturb on its own is a toggle tile with the bell-off icon.

### Displays, power and project

Figma: v0.3 › Displays + Power (tiles) and Sound + System pages.

- System page, Displays card: a row per display (icon, name, Doto value, inline slider), then PROJECT as a 2×2 grid of choices (144 px, radius 12; picked = inverted plate).
- Power card: Efficiency / Balanced / Performance as a segmented control with icons, the battery row with a big Doto %, and Energy saver as status text plus an "Open Settings" pill. Apps can't switch Energy saver.
- **Night light is parked:** Windows has no supported API for it. Its designs stay in Figma, dimmed, for later.

### Per-app output

Figma: v0.3 › Sound + System pages.

- Under each app's name in the mixer: speaker glyph + device name + a down chevron, in text-tertiary. An app moved off Default shows it in `accent-text`.
- Clicking it opens a list in place, under that row: "Default (Headphones)" first, then every output. The picked one is the inverted pill with a check.
- When any app is moved, the card ends with "1 app not on Default" and a secondary "Reset all".

### Other looks

Figma: v0.3 › Liquid + Windows looks.

The same layouts and sizes in Liquid and Windows. What changes:

- **Cards:** Liquid uses the preset's tint, 18 px radius and a lit top rim. Windows uses flat #2B2B2B, 8 px radius and a hairline.
- **Type:** numbers are in Segoe UI Variable Display Semibold, not Doto.
- **Accent:** "on" tiles and selections fill with the look's accent (the Liquid preset's, or Windows blue #4CC2FF).
- **Page dots and page bar:** the same plain dots and bar in every look (ink at 30 %, the current page full); the drop target uses the look's accent.
- **Analogue clock:** line ticks and solid hands instead of dots.

## Open questions

1. **Fonts in Figma.** Install OCR-A for Figma (from `src/assets/fonts`), then set the `font/ui` variable so the mockups use the real face.
2. **Icons.** The app uses Segoe Fluent glyphs; the Figma set is Traywise's own style. Should the app switch to the Figma icons (as inline SVG) one day, or should Figma follow Fluent?
3. **Clock style:** one global setting, or per tile?

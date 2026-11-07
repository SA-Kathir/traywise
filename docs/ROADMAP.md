# Roadmap and status

The single place to see where Traywise is and where it's going. Update it when a release ships, a PR lands, or a plan changes. Last updated: 2026-11-03.

## Released

| Version | Date | Headline |
|---|---|---|
| 0.1.0 | 2026-10-26 | First release: tray panel, Liquid and Windows looks, mixer, audio devices, keep awake, system monitor, quick toggles, auto-updates |
| 0.2.0 | 2026-10-31 | Dot look (the new default), customisable home board, Now playing, Wi-Fi, Bluetooth, opening from the screen edge |
| 0.3.0 | 2026-11-03 | Displays, Power, Clock and Focus modules, per-app audio output, board pages, a new open and close animation, the Liquid look renamed Tint, a new app icon |
| 0.3.1 | 2026-11-07 | Tray menu and accent icon, Esc, Pomodoro Focus (arc or hourglass), choose the microphone, Shortcuts in Settings, larger Project tiles, a calmer System monitor, in-app updates with progress |

Details: `CHANGELOG.md`. Releases: https://github.com/SA-Kathir/traywise/releases

## In progress

**Glass look:** a fourth look, with real blur behind the panel (Windows' host backdrop brush; a throwaway test proved it works on the panel's window setup) and Apple-style translucent cards. The design is done in Figma and the build starts once 0.3.1 is on `dev`. Open question: its tint adapts to how bright the screen behind it is, which would extend the one narrow "never read the desktop" exception in CLAUDE.md; the owner decides.

Otherwise nothing. Work goes on a `feature/<name>` or `fix/<name>` branch from `dev` and is merged by pull request; when several run at once, each gets its own git worktree (a sibling folder like `../tw-<name>`), removed once its branch is merged.

## Next: v0.4.0 (proposed, not started)

- **The Glass look** (see In progress).
- **Scenes:** one tap sets brightness, power mode, Do not disturb, volume and mic mute together ("Meeting", "Gaming", "Night").
- **A privacy indicator:** which apps use the mic or camera now and recently (Windows' consent store).
- **Display quick-switches:** HDR, refresh rate, resolution.
- **Hotspot and airplane mode tiles.**
- **`traywise://` links:** e.g. `traywise://toggle/wifi`, `traywise://scene/meeting`, so launchers and scripts can drive Traywise.
- **GPU usage** in the System monitor (Windows' GPU engine counters, as Task Manager uses; measure the cost first).
- **Keep awake triggers:** stay awake while an external display is connected or while chosen apps run.
- **Open on the monitor under the mouse** (the panel and the edge strip are primary-monitor only today).
- **Pinned tiles:** keep a tile (clock, now playing, Focus) on screen without the whole panel.

## Later: v0.5.0, then 1.0

- **Platform:** a module SDK (docs + a `pnpm new-module` template), translations, settings and board import/export, an accessibility pass, and CI checks that fail on speed or memory regressions.
- **1.0 means:** all modules stable, a documented module API, no known crashes, and a clean-install test passing. Code signing and a winget listing come later.

## Product boundaries (decided)

- **No launcher features:** clipboard history, snippets, notes, calculator, web search, file search. (The Focus module's Pomodoro-style timer is in scope: it is Focus with focus/rest cycles, not a general timer app.) Launchers can drive Traywise through `traywise://` links instead.
- **Out of scope:**
  - window snapping (PowerToys does it)
  - uninstallers/cleaners
  - temperatures/fans (they need a kernel driver, which antivirus flags)
- **No paid services for now:** ship unsigned with the SmartScreen note.
- **Privacy:** local only, no telemetry; the network is only used for update checks. Never read the desktop or wallpaper (one narrow exception, documented in CLAUDE.md).

## Open owner decisions

- Clean-machine test in Windows Sandbox (`docs/testing-release.md`) still hasn't been run: 0.3.0 shipped after a friend tried it on another laptop instead.
- Volume and brightness widgets need a redesign, in every look: the 2 × 2 fills the whole tile with the level, which looks heavy and gives nothing to grab, and the 1 × 1 shows a value you cannot change. Designed for the Glass look (to apply to the other looks and build): the 1 × 1 is a ring at rest and a plus/value/minus stepper on hover, with scroll to change and click to mute; the 2 × 2 is a draggable dial for one device, and a set of vertical faders (one per app or display, main one outlined) when there are several. The 2 × 1 and 4 × 1 keep the filled bar for now.
- Power mode and Battery both use the lightning-bolt icon, which is confusing as two 1 × 1 tiles. Power mode should get its own icon (a gauge in the Glass designs).

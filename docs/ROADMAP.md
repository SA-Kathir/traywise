# Roadmap and status

The single place to see where Traywise is and where it's going. Update it when a release ships, a PR lands, or a plan changes. Last updated: 2026-11-14.

## Released

| Version | Date | Headline |
|---|---|---|
| 0.1.0 | 2026-10-26 | First release: tray panel, Liquid and Windows looks, mixer, audio devices, keep awake, system monitor, quick toggles, auto-updates |
| 0.2.0 | 2026-10-31 | Dot look (the new default), customisable home board, Now playing, Wi-Fi, Bluetooth, opening from the screen edge |
| 0.3.0 | 2026-11-03 | Displays, Power, Clock and Focus modules, per-app audio output, board pages, a new open and close animation, the Liquid look renamed Tint, a new app icon |
| 0.3.1 | 2026-11-07 | Tray menu and accent icon, Esc, Pomodoro Focus (arc or hourglass), choose the microphone, Shortcuts in Settings, larger Project tiles, a calmer System monitor, in-app updates with progress |
| 0.4.0 | 2026-11-14 | The Glass look, dials and sliders for Volume and Brightness, a gallery for adding and resizing tiles, GPU usage, refresh rate, hotspot and airplane tiles, Docked keep awake, every screen supported |

Details: `CHANGELOG.md`. Releases: https://github.com/SA-Kathir/traywise/releases

## In progress

Nothing. Work goes on a `feature/<name>` or `fix/<name>` branch from `dev` and is merged by pull request; when several run at once, each gets its own git worktree (a sibling folder like `../tw-<name>`), removed once its branch is merged.

## Next: a minor update, then v0.5.0 (proposed, not started)

- **A privacy indicator:** which apps use the mic or camera now. A first version was built (mic from the audio engine, camera from Windows' camera activity monitor, both worked) and set aside before 0.4.0 because the 1×1 and 2×1 tiles were too cramped to show the apps; the branch `feature/privacy-indicator` and a roomier 1×1 / 2×1 / 2×2 design (Figma page "v0.4 · Privacy tile") are the starting point.
- **Scenes:** one tap sets brightness, power mode, Do not disturb, volume and mic mute together ("Meeting", "Gaming", "Night").
- **`traywise://` links:** e.g. `traywise://toggle/wifi`, `traywise://scene/meeting`, so launchers and scripts can drive Traywise.
- **HDR and resolution quick-switches** (Windows has no simple public call for HDR; refresh rate shipped in 0.4.0).
- **Keep awake while chosen apps run** (only if it can stay event-driven).
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

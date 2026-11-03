# Roadmap and status

The single place to see where Traywise is and where it's going. Update it when a release ships, a PR lands, or a plan changes. Last updated: 2026-11-03.

## Released

| Version | Date | Headline |
|---|---|---|
| 0.1.0 | 2026-10-26 | First release: tray panel, Liquid and Windows looks, mixer, audio devices, keep awake, system monitor, quick toggles, auto-updates |
| 0.2.0 | 2026-10-31 | Dot look (the new default), customisable home board, Now playing, Wi-Fi, Bluetooth, opening from the screen edge |
| 0.3.0 | 2026-11-03 | Displays, Power, Clock and Focus modules, per-app audio output, board pages, a new open and close animation, the Liquid look renamed Tint, a new app icon |

Details: `CHANGELOG.md`. Releases: https://github.com/SA-Kathir/traywise/releases

## In progress

Nothing. Work goes on a `feature/<name>` or `fix/<name>` branch from `dev` and is merged by pull request; when several run at once, each gets its own git worktree (a sibling folder like `../tw-<name>`), removed once its branch is merged.

## Next: v0.4.0 (proposed, not started)

- **Scenes:** one tap sets brightness, power mode, Do not disturb, volume and mic mute together ("Meeting", "Gaming", "Night").
- **A privacy indicator:** which apps use the mic or camera now and recently (Windows' consent store).
- **Display quick-switches:** HDR, refresh rate, resolution.
- **Hotspot and airplane mode tiles.**
- **`traywise://` links:** e.g. `traywise://toggle/wifi`, `traywise://scene/meeting`, so launchers and scripts can drive Traywise.

## Later: v0.5.0, then 1.0

- **Platform:** a module SDK (docs + a `pnpm new-module` template), translations, settings and board import/export, an accessibility pass, and CI checks that fail on speed or memory regressions.
- **1.0 means:** all modules stable, a documented module API, no known crashes, and a clean-install test passing. Code signing and a winget listing come later.

## Product boundaries (decided)

- **No launcher features:** clipboard history, snippets, notes, timer/Pomodoro, calculator, web search, file search. Launchers can drive Traywise through `traywise://` links instead.
- **Out of scope:**
  - window snapping (PowerToys does it)
  - uninstallers/cleaners
  - temperatures/fans (they need a kernel driver, which antivirus flags)
- **No paid services for now:** ship unsigned with the SmartScreen note.
- **Privacy:** local only, no telemetry; the network is only used for update checks. Never read the desktop or wallpaper (one narrow exception, documented in CLAUDE.md).

## Open owner decisions

- Tint look (the old Liquid): **Background** Tinted or Transparent is still undecided (both are Settings options). **Tiles** is decided: Single hue is the default, Shaded stays a choice.
- Clean-machine test in Windows Sandbox (`docs/testing-release.md`) still hasn't been run: 0.3.0 shipped after a friend tried it on another laptop instead.
- Opening from the edge with only the desktop in front: fixed in 0.3.0 (the desktop counted as a full-screen app), but not yet confirmed on a bare desktop.

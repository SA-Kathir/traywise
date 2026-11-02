# Roadmap and status

The single place to see where Traywise is and where it's going. Update it when a release ships, a PR lands, or a plan changes. Last updated: 2026-11-01.

## Released

| Version | Date | Headline |
|---|---|---|
| 0.1.0 | 2026-10-26 | First release: tray panel, Liquid and Windows looks, mixer, audio devices, keep awake, system monitor, quick toggles, auto-updates |
| 0.2.0 | 2026-10-31 | Dot look (the new default), customisable home board, Now playing, Wi-Fi, Bluetooth, opening from the screen edge |

Details: `CHANGELOG.md`. Releases: https://github.com/SA-Kathir/traywise/releases

## In progress: v0.3.0, "complete the Control Centre"

Built in parallel, each in its own git worktree (a sibling folder next to `traywise/`) on its own branch, merged into `dev` by PR:

| Feature | Branch | Worktree | State (2026-11-01) |
|---|---|---|---|
| Displays + Power: brightness per display (WMI + DDC/CI), power mode, battery | `feature/displays-power` | `../tw-displays` | PR #5 open. External monitors and "Project to a second screen" are untested. |
| Per-app audio output: a device picker per app in the mixer | `feature/per-app-output` | `../tw-audio-out` | PR #6 open. It conflicts with #5 in `Cargo.toml` and the mixer's `Panel.tsx`, so whichever merges second needs a rebase. |
| Clock tile (dot-matrix, second time zone) + Focus / Do not disturb | `feature/clock-focus` | `../tw-clock` | Built and in owner review. Speed not measured yet; no PR yet. DND reading and switching work through a private Windows interface. |
| Board pages: several swipeable boards, page dots, drop a tile on a dot to move it, up to 5 pages | `feature/board-pages` | `../tw-pages` | In progress. Settings save pages; a 0.2 board migrates to page 1. |
| Design system synced with 0.2.0, plus the 0.3.0 designs | `docs/design-system-v0.3` | `../tw-design` | PR #7 open (only `docs/design-system.md`). |

Once every PR is merged into `dev`, a release follows the usual steps (CONTRIBUTING.md › Branches): add a CHANGELOG section, merge `dev` into `main`, tag `v0.3.0`, and publish the draft release. After a branch is merged, remove its worktree with `git worktree remove ../tw-<name>`, and delete the branch.

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
- Clean-machine test in Windows Sandbox (`docs/testing-release.md`) hasn't been run yet.

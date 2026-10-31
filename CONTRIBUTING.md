# Contributing

Thanks for helping. Traywise has two priorities, in this order: **fast and lean, with no bloat**, then **a clean liquid-glass look**. Changes that cost CPU or memory while the panel is hidden are unlikely to be merged.

## Build

You need Node.js 22+, pnpm, stable Rust and the [Tauri prerequisites for Windows](https://tauri.app/start/prerequisites/).

```bash
pnpm install
pnpm tauri dev
```

Before opening a pull request, run the same checks as CI:

```bash
pnpm build
cd src-tauri && cargo clippy --all-targets --all-features -- -D warnings && cargo test
```

`src/bindings.ts` is generated from Rust (tauri-specta) whenever a debug build starts. Never edit it by hand; `cargo test` fails if it's out of date.

Measure speed and memory with release builds (`pnpm tauri build --no-bundle`); debug builds give misleading numbers.

## How the code is laid out

- `src/`: the React UI. `src/shell/` is the panel frame, theme and settings; `src/modules/<id>/` is each module's UI.
- `src-tauri/src/`: Rust, everything that talks to Windows. `shell/` is the tray and panel; `modules/<id>/` is each module.

## Adding a module

A module is one feature that can be switched on or off.

1. **Rust:** add a folder in `src-tauri/src/modules/`, implement the `Module` trait from `src-tauri/src/module.rs` (`start`, `stop`, and optionally `apply_settings` and `panel_visible`), add an id to `ModuleId` and the module to `registry()`.
2. **UI:** add `src/modules/<id>/index.ts` with its name, icon and lazy-loaded components, and list it in `src/modules/index.ts`.

Rules:
- A module that's switched off is never started: no threads, hooks or subscriptions. Creating it must do nothing.
- Event-driven, never polling. When nothing is happening, CPU should be about 0%.
- Its UI is lazy-loaded, so a disabled module's code isn't loaded.
- Modules are built in. No third-party plugins.

## Branches

| Branch | Holds | Who writes to it |
|---|---|---|
| `main` | Released versions only. Every commit on it is a release, tagged `vX.Y.Z`. | Release merges from `dev` only |
| `dev` | The next release, as it comes together | Merged pull requests only |
| `feature/<name>` | One feature or module | You, branched from `dev` |
| `fix/<name>` | One bug fix | You, branched from `dev` |

- Branch from `dev`, and open your pull request against `dev`. Branches for docs, CI or chores follow the same pattern: `docs/<name>`, `ci/<name>`, `chore/<name>`.
- **To release:** give `CHANGELOG.md` a `## X.Y.Z (date)` section, merge `dev` into `main`, then tag the merge `vX.Y.Z`. The tag triggers the release build, which puts that section on the GitHub release as its notes (`scripts/release-notes.mjs`; the build stops if the section is missing).
- **Urgent fix for a released version:** branch `fix/<name>` from `main`, release it from `main` as a patch, then merge `main` back into `dev`.

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <what changed, in the imperative>
```

- **type** is one of:

  | Type | For |
  |---|---|
  | `feat` | a new feature |
  | `fix` | a bug fix |
  | `perf` | faster or leaner |
  | `refactor` | the same behaviour, better code |
  | `style` | looks only (CSS) |
  | `docs` | documentation |
  | `test` | tests |
  | `build` | dependencies, Tauri config |
  | `ci` | workflows |
  | `chore` | anything else |

- **scope** (optional) is the area: a module (`mixer`, `keep-awake`, `wifi`, `now-playing`…), or `board`, `shell`, `settings`, `dot-look`, `release`.
- Keep the summary line under 72 characters, in lowercase after the colon, and with no full stop. Explain the why in the body if it isn't obvious.
- For a breaking change, put `!` after the type or scope (`feat(settings)!: …`) and add a `BREAKING CHANGE:` line to the body.

Examples:
- `feat(board): swipeable board pages`
- `fix(mixer): keep an app's volume after it restarts`
- `docs: explain the branching model`

Pull request titles follow the same format.

## Pull requests

- Target `dev`, never `main`.
- Keep them small: one module or one concern per PR.
- Explain what changes for the user, and how you tested it on Windows 11.
- By contributing, you agree your work is licensed under GPL-3.0-or-later.

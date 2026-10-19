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

## Pull requests

- Keep them small: one module or one concern per PR.
- Explain what changes for the user, and how you tested it on Windows 11.
- By contributing, you agree your work is licensed under GPL-3.0-or-later.

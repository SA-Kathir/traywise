# Traywise

A free, open-source (GPL-3.0-or-later) Windows 11 tray toolkit: one tray icon and one panel, with features as modules you switch on or off. It's inspired by vorssaint/vorssaint-utils (macOS). Don't use Vorssaint's name, icon or look, which are trademarked, and don't copy its code (it's Swift).

Priorities, in order: **1. fast and lean, with no bloat; 2. a clean liquid-glass look.**

## Working with the owner

- The owner knows TypeScript and React. Rust and the Windows APIs are a black box to them, so explain every change in plain terms: what it does on their PC, not just the code.
- Keep diffs small and reviewable (Ponytail is enabled for this project). One module or concern per change.
- Commit as you go, in small atomic commits (one concern each). No `Co-Authored-By` or other AI attribution in commit messages. Never push or create a GitHub repo without asking.

## Stack

- **Tauri 2**: shell, tray, windows, installer (NSIS only)
- **Rust**: everything that talks to Windows, using Microsoft's `windows` crate
- **React + TypeScript + Vite**: everything you see
- **pnpm**
- **Generated Rust → TypeScript types** (tauri-specta or ts-rs). The UI never hand-writes the shape of data that comes from Rust.

## Layout (target)

```
src/                      React (owner's side)
  shell/                  panel frame, Liquid theme, navigation, settings page
  modules/<id>/           index.ts (id, name, icon, lazy panel) + its components
  bindings.ts             GENERATED from Rust. Never edit by hand.
src-tauri/src/
  main.rs / lib.rs        startup, tray, loads enabled modules
  module.rs               the module contract: id, start(), stop()
  shell/                  tray.rs, panel.rs (show/hide/position), webview.rs
  modules/<id>/           one folder per module
```

Module rules:
- A disabled module is never started in Rust (no threads, hooks or subscriptions), and its UI is lazy-loaded, so its code isn't loaded at all.
- Modules are built in. No third-party plugin loading; that would mean running other people's native code.
- Event-driven, never polling. When nothing is happening, CPU should be about 0%.

## Proven in ../glass-spike (read its README and src-tauri/src/lib.rs)

Measured numbers from the spike, release build: about 65 ms to open, 0.2% of a CPU core while hidden, about 140–150 MB of RAM (working set) once hidden for a minute or more. The spike's old "76 MB" was an early reading taken ~30 s after hiding, not the steady state. Keep open time and CPU there; the RAM target is paused for now (WebView2 browser flags cut it to ~125–130 MB, not adopted yet).

- **Look: "Liquid".** The app draws its own still background (colour-blob presets as CSS variables in `src/shell/liquid.css`: Ocean, Graphite, Forest, Plum, Ember, Rose; one muted hue each, subtle, never bright; the owner rejected the multicolour Dusk preset), and the cards and controls on top are glass: `backdrop-filter: blur(14px) saturate(1.7)`, a specular `::before` highlight, and rim shadows. The preset class sits on `.panel`, so everything can use `--accent`.
- **Native Mica/Acrylic doesn't work** on the owner's PC, even with fixes. Don't go back to it.
- **Never read the desktop or wallpaper.** The owner rejected this for privacy.
- **The background doesn't move.** It used to drift for ~10 s after each open (continuous motion costs about 35% of a core), but blurred behind the glass the motion wasn't noticeable, so the owner agreed to drop it. The SVG refraction filter (`url(#liquid-refract)` in `backdrop-filter`) was dropped too: it was barely visible and is the likely cause of stale blue blocks seen in WebView2.
- **When the panel hides, call `controller.SetIsVisible(false)` on WebView2.** Hiding the window alone keeps WebView2 rendering, which cost about 39% of a core while hidden.
- **Set WebView2 `MemoryUsageTargetLevel` to Low while hidden and Normal before showing** (through `ICoreWebView2_19`). It trims RAM right after hiding (dips to ~65–75 MB), but most of it comes back within a minute or two.
- **Create the panel window in code** (`WebviewWindowBuilder`), not in tauri.conf.json: borderless, transparent, always on top, no taskbar entry, rounded corners through DWM (`backdrop.rs`).
- **Panel behaviour:**
  - Left-click on the tray icon toggles it; right-click opens a menu with Quit.
  - Losing focus hides it.
  - Ignore a tray click within 250 ms of a focus-loss hide; it's the same click.
  - Alt+F4 hides instead of quitting.
- **Position:** bottom-right of the primary monitor's work area with a 12 px margin. Physical pixels; convert when using logical builder APIs. The display is scaled 125%.
- **Bundle identifier:** letters, numbers, hyphens and dots only. An underscore broke the build.
- **Measure in release builds** (`pnpm tauri build --no-bundle`). Debug builds give misleading speed and memory numbers. The spike's `scripts/measure-memory.ps1` sums the app plus its WebView2 child processes.

## Commands

- Dev, with hot reload: `pnpm tauri dev`
- Release exe: `pnpm tauri build --no-bundle`, which produces `src-tauri/target/release/traywise.exe`
- Measure: `pnpm tauri build --no-bundle --features autotest`, run `traywise.exe > log.txt` (it opens/closes itself 10 times, printing `panel opened in N ms`), then `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/measure-memory.ps1`. Wait 2+ minutes: RAM dips right after hiding, then settles higher.
- Bindings: `src/bindings.ts` (tauri-specta) is rewritten whenever a debug build starts (`pnpm tauri dev`), and `cargo test` fails if it's stale. New commands/events go in `bindings()` in `src-tauri/src/lib.rs`.
- CI: `.github/workflows/ci.yml` (Windows runner) runs `pnpm build`, clippy with `-D warnings`, `cargo test` and the release build.
- Type-check: `pnpm exec tsc --noEmit`. Rust: `cargo build`, `cargo clippy`, `cargo test` in `src-tauri`.
- Rust may not be on PATH in a fresh shell: `export PATH="$HOME/.cargo/bin:$PATH"`.

## Environment notes

- Windows 11 (build 26200). Long paths are disabled, and the project path is about 45 characters, so keep nesting shallow.
- The shell is Git Bash. PowerShell scripts need `-NoProfile -ExecutionPolicy Bypass -File`.

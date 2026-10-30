# Traywise

A free, open-source (GPL-3.0-or-later) Windows 11 tray toolkit: one tray icon and one panel, with features as modules you switch on or off. It's inspired by vorssaint/vorssaint-utils (macOS). Don't use Vorssaint's name, icon or look, which are trademarked, and don't copy its code (it's Swift).

Priorities, in order: **1. fast and lean, with no bloat; 2. a clean liquid-glass look.**

## Working with the owner

- The owner knows TypeScript and React. Rust and the Windows APIs are a black box to them, so explain every change in plain terms: what it does on their PC, not just the code.
- Keep diffs small and reviewable (Ponytail is enabled for this project). One module or concern per change.
- Commit as you go, in small atomic commits (one concern each). No `Co-Authored-By` or other AI attribution in commit messages. Never push or create a GitHub repo without asking.
- **Branches (see CONTRIBUTING.md):**
  - Never commit to `main`; it holds released versions only, each tagged `vX.Y.Z`.
  - New work goes on `feature/<name>` or `fix/<name>`, branched from `develop`, and is merged back into `develop` by pull request.
  - Check `git branch --show-current` before your first commit.

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

- **Three looks, picked at first run and in Settings** (`look-dot` / `look-liquid` / `look-windows` on `.panel`, styles in `src/shell/liquid.css`, Dot's overrides in `src/shell/dot.css`):
  - **Dot** (default): built from `docs/design-system.md`'s tokens (`--tw-*`). Monochrome, one accent preset, bundled Doto (OFL) for numbers and the wordmark only, dot-grid or plain background, follows Windows light/dark. Blur only on floating layers (the Add tiles tray).
  - **Liquid:** cards tinted in a colour preset (Ocean, Graphite, Forest, Plum, Ember, Rose; one muted hue each, never bright; the owner rejected the multicolour Dusk preset) with a thin edge of the same hue; the tab bar, icon buttons and titles use the preset too. Two Settings choices the owner hasn't settled yet: **Background** Tinted (a deep shade of the preset) or Transparent (`.clear`: cards float over the desktop; the title row gets its own card), and **Tiles** Shaded (lighter top left, fading darker) or Single hue (`.flat`). No blobs, no white specular highlight, no `backdrop-filter`: the owner found blobs-behind-glass random and "vibe-coded". The look's classes are on `.panel` and on `html` (so the scrollbar's strip matches).
  - **Windows:** flat like Windows 11's flyouts, follows Windows light/dark mode, Windows' default blue. Text and light fills use `rgb(var(--ink) / a)`, so light mode only flips `--ink`.
- **The panel has a fixed height (640 logical px)**; pages that don't fit scroll. The owner found a window that resized to each page's content jarring.
- **Native Mica/Acrylic doesn't work** on the owner's PC, even with fixes. Don't go back to it.
- **Never read the desktop or wallpaper.** The owner rejected this for privacy.
- **The background doesn't move.** It used to drift for ~10 s after each open (continuous motion costs about 35% of a core), but blurred behind the glass the motion wasn't noticeable, so the owner agreed to drop it. The SVG refraction filter (`url(#liquid-refract)` in `backdrop-filter`) was dropped too: it was barely visible and is the likely cause of stale blue blocks seen in WebView2.
- **When the panel hides, call `controller.SetIsVisible(false)` on WebView2.** Hiding the window alone keeps WebView2 rendering, which cost about 39% of a core while hidden.
- **Set WebView2 `MemoryUsageTargetLevel` to Low while hidden and Normal before showing** (through `ICoreWebView2_19`). It trims RAM right after hiding (dips to ~65–75 MB), but most of it comes back within a minute or two.
- **Create the panel window in code** (`WebviewWindowBuilder`), not in tauri.conf.json: borderless, transparent, always on top, no taskbar entry. Windows 11's rounded corners, border and shadow whenever the panel has a background (`framed`/`frame` in `shell/panel.rs`); transparent Liquid turns all three off, since rounded corners alone bring the system shadow and border. The frame's shadow takes 18 x 10 physical px of the window, so `place` adds it back (Windows misreports a frame right after resizing an unframed window, so it only measures when framed).
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
- UI logic tests: `pnpm test` (plain scripts Node runs as TypeScript, no test framework).
- Type-check: `pnpm exec tsc --noEmit`. Rust: `cargo build`, `cargo clippy`, `cargo test` in `src-tauri`.
- Rust may not be on PATH in a fresh shell: `export PATH="$HOME/.cargo/bin:$PATH"`.

## Environment notes

- Windows 11 (build 26200). Long paths are disabled, and the project path is about 45 characters, so keep nesting shallow.
- The shell is Git Bash. PowerShell scripts need `-NoProfile -ExecutionPolicy Bypass -File`.

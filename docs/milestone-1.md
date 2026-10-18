# Milestone 1: app shell + volume mixer

Read `CLAUDE.md` first. It has the stack, the layout, the module rules and the hard-won Windows quirks from `../glass-spike`. Build in small steps that can be reviewed one at a time. After each step, explain in plain terms what changed and how to try it.

## Steps

1. **Shell**
   - The panel window is created in code, plus the tray icon (left-click toggles, right-click → Quit).
   - Show/hide, using every trick from the spike: `SetIsVisible`, memory trim, the 250 ms reopen guard, hide on focus loss, Alt+F4 hides.
   - Position: bottom-right of the work area.
2. **Liquid theme**
   - Port the spike's Liquid CSS into `src/shell/`: the background presets and the glass styles for cards and controls.
   - Background motion only after opening, and make it noticeably more visible than in the spike (the owner's feedback), still short.
3. **Typed bindings**: Rust → TypeScript type generation (`bindings.ts`), with a build check that fails when the generated file is stale.
4. **Module system**
   - `module.rs` contract (id, start, stop) and a registry. On the UI side, each module registers an id, name, icon and a lazy-loaded panel.
   - Settings are saved to `%APPDATA%\Traywise\settings.json`: enabled modules, theme preset, motion.
   - A disabled module is never started.
5. **Settings page**: switch modules on and off, choose the background preset, choose the motion, and start with Windows (Tauri autostart plugin).
6. **Volume mixer module** (`modules/volume_mixer` + `src/modules/volume-mixer`)
   - Lists every app playing audio on the default output device: name, icon, volume, mute. Shows the master volume at the top.
   - Sliders set per-app volume, throttled to about 60 calls per second while dragging. Mute toggles.
   - Stays in sync with changes made elsewhere (Windows' own mixer, media keys) through Windows audio session events. No polling.
   - Handles apps that start or stop playing audio, and changes of the default device.
   - Reference: EarTrumpet (MIT, C#) for which Windows audio APIs to use (`IAudioSessionManager2`, `IAudioSessionNotification`, `IAudioSessionEvents`).
7. **Checks**
   - Rust unit tests where logic isn't a raw Windows call.
   - `cargo clippy` clean, and `tsc` clean.
   - A GitHub Actions workflow (Windows runner) that runs the build and the checks. Write it, but the repo doesn't exist on GitHub yet, so don't push anything.

## Done when

- The release build opens the panel from the tray in under 100 ms. While hidden it uses at most about 1% of a CPU core. (The ~100 MB RAM target is paused: it came from an early spike reading; the real baseline is ~140–150 MB.) Measure with the spike's scripts and report the numbers.
- The volume mixer controls real apps (try it with Spotify or a browser), and it reflects changes made in Windows' own mixer.
- Turning the mixer off in settings means none of its code runs (show the evidence).
- Nothing is committed or pushed without the owner's OK.

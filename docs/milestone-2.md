# Milestone 2: more modules (functionality first)

**Status: done (2026-10-19).** All four modules, Quick toggles included, are built and were checked on the owner's PC. A first UI pass happened too (home tiles, module pages, six colour presets), pulled forward from milestone 3. Notes at the end.

Goal: make Traywise a toolkit rather than a nicer EarTrumpet. **Functionality comes first.** The owner isn't happy with the UI yet, the mixer's included, but polish is milestone 3. For now, reuse the existing Liquid components so new UI looks consistent, and don't spend time on visual redesign. Note any UI ideas in `docs/ui-notes.md` for later.

Read `CLAUDE.md` first. Same working rules as milestone 1:
- Small, reviewable steps.
- Explain each one in plain terms and stop for the owner's OK.
- Commit only when the owner says so; never push.

## Shared groundwork

- **Several modules in one panel.** For now, stack the enabled modules' cards vertically, in the order from settings, and let the panel scroll if it gets tall. The real navigation design belongs to milestone 3; keep this simple and easy to replace.
- **Shared audio helpers.** The mixer and Audio devices both need Core Audio (device enumeration, endpoint volume). Put the shared pieces in one Rust helper. Each module still starts, stops and is switched on or off independently.

## Modules, in order

### 1. Keep awake (`keep_awake`)
- **Modes:** off; on until turned off; on for 30 min / 1 h / 2 h; on while plugged in. An option also keeps the display on.
- **How it works:** `SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED [| ES_DISPLAY_REQUIRED])`, held by the module and released on stop, quit and crash. Show the time left for timed modes.
- **Verify:** `powercfg /requests` lists Traywise while it's on, and doesn't after it's switched off or Traywise quits.

### 2. Audio devices (`audio_devices`)
- **Outputs:**
  - List them and show the current default.
  - Clicking one makes it the default for every role. This uses the undocumented-but-stable `IPolicyConfig` interface; see EarTrumpet or SoundSwitch.
  - Follow device changes (plugging in, unplugging, default changed elsewhere) with `IMMNotificationClient`. No polling.
- **Microphones:**
  - "Mute all microphones" sets the endpoint mute on every capture device.
  - A global shortcut toggles it, using Tauri's global-shortcut plugin; the default key is configurable in settings.
  - Clear muted/unmuted state in the panel.
  - Newly plugged-in mics follow the current mute-all state.
- **Verify:** switching output moves the audio of a playing app. Windows' Sound settings show the mics muted. The shortcut works with the panel closed.

### 3. System monitor (`system_monitor`)
- **Shows:** total CPU %, RAM used/total, network download/upload rate, battery % (charging or time left), free space on the system drive.
- **Sampling:** once a second, **only while the panel is visible**. Zero work while hidden, so idle CPU stays near 0%.
- **History:** a small 60-second graph for CPU and network, kept only while visible.
- **Library:** `sysinfo` (the spike already used it). GPU and temperatures are out of scope; temperatures need a kernel driver.
- **Verify:** the numbers match Task Manager within reason. CPU while hidden doesn't change with this module on.

### 4. Stretch, only if 1–3 are done: Quick toggles (`quick_toggles`)
- App/system light–dark theme (registry, then broadcast `WM_SETTINGCHANGE`).
- Show/hide desktop icons.
- Lock the screen.

## Done when

- Each module works as described and is verified on the owner's PC.
- A switched-off module never starts: no threads, no execution-state request, no shortcut registered, and its UI file is never loaded.
- With all modules on, the release build still opens in under 100 ms and stays at about 1% CPU or less while hidden. Report the numbers.
- Unit tests cover the logic that isn't a direct Windows call, such as keep-awake timers and rate calculations.
- clippy and tsc are clean, and the CI steps pass locally.

## Result

- **Measured** (release build, all five modules on): the panel opens in 42–65 ms. Hidden CPU settles to 0.0–0.1% of a core after the first minute. RAM is about 172 MB working set; RAM has no target for now.
- **Changed from the plan:**
  - Keep awake uses `PowerSetRequest` rather than `SetThreadExecutionState`. It's the same effect, but not tied to one thread.
  - On Modern Standby laptops, Windows sleeps when the screen turns off, whatever apps ask. The panel says so.
  - The timer is a −/+ stepper (1 min to 12 h) instead of 30 min / 1 h / 2 h.
  - System monitor reads CPU from `% Processor Utility`, as Task Manager does, not from sysinfo.
- **Dropped:** background motion and the SVG refraction filter (see CLAUDE.md).
- **Left for later:** the ideas in `docs/ui-notes.md`.


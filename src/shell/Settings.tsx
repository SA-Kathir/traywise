import { useEffect, useState } from "react";
import { commands, type ModuleId, type Preset, type Settings } from "../bindings";
import { modules } from "../modules";

const PRESETS: { id: Preset; label: string }[] = [
  { id: "ocean", label: "Ocean" },
  { id: "graphite", label: "Graphite" },
  { id: "forest", label: "Forest" },
  { id: "plum", label: "Plum" },
  { id: "ember", label: "Ember" },
  { id: "rose", label: "Rose" },
];

const MODULE_IDS = Object.keys(modules) as ModuleId[];

export function SettingsPage({ settings, onChange }: { settings: Settings; onChange: (next: Settings) => void }) {
  // null until Windows has answered.
  const [autostart, setAutostart] = useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    commands.getAutostart().then((r) => (r.status === "ok" ? setAutostart(r.data) : setError(r.error)));
  }, []);

  // Show the change at once; undo it if Rust couldn't save it.
  async function save(next: Settings) {
    onChange(next);
    const r = await commands.setSettings(next);
    if (r.status === "error") {
      onChange(settings);
      setError(r.error);
    } else {
      setError(null);
    }
  }

  function toggleModule(id: ModuleId, on: boolean) {
    // Keep the registry's order, so the main view doesn't reshuffle.
    const enabledModules = MODULE_IDS.filter((m) => (m === id ? on : settings.enabledModules.includes(m)));
    save({ ...settings, enabledModules });
  }

  async function toggleAutostart(on: boolean) {
    const r = await commands.setAutostart(on);
    if (r.status === "ok") setAutostart(on);
    else setError(r.error);
  }

  return (
    <>
      <section>
        <h2>Modules</h2>
        {MODULE_IDS.map((id) => (
          <label key={id} className="row">
            <span className="icon" aria-hidden>
              {modules[id].icon}
            </span>
            <span className="grow">{modules[id].name}</span>
            <input
              type="checkbox"
              checked={settings.enabledModules.includes(id)}
              onChange={(e) => toggleModule(id, e.target.checked)}
            />
          </label>
        ))}
      </section>

      <section>
        <h2>Colour</h2>
        <div className="swatches" role="radiogroup" aria-label="Colour">
          {PRESETS.map((p) => (
            <button
              key={p.id}
              role="radio"
              aria-checked={settings.preset === p.id}
              className={settings.preset === p.id ? "on" : ""}
              onClick={() => save({ ...settings, preset: p.id })}
            >
              {/* Its own preset class, so the swatch shows that preset's colours. */}
              <span className={`swatch bg-${p.id}`} aria-hidden />
              {p.label}
            </button>
          ))}
        </div>
      </section>

      {settings.enabledModules.includes("audio-devices") && (
        <section>
          <h2>Shortcuts</h2>
          <div className="row">
            <span className="grow">Mute all microphones</span>
            <ShortcutPicker keys={settings.micShortcut} onChange={(micShortcut) => save({ ...settings, micShortcut })} />
          </div>
        </section>
      )}

      <section>
        <h2>System</h2>
        <label className="row">
          <span className="grow">Start with Windows</span>
          <input
            type="checkbox"
            disabled={autostart === null}
            checked={autostart ?? false}
            onChange={(e) => toggleAutostart(e.target.checked)}
          />
        </label>
      </section>

      {error && <p className="error">{error}</p>}
    </>
  );
}

const MODIFIER_KEYS = ["Control", "Alt", "Shift", "Meta"];

/** Click, then press the new key combination (with Ctrl, Alt, Shift or Win). Esc cancels. */
function ShortcutPicker({ keys, onChange }: { keys: string; onChange: (keys: string) => void }) {
  const [listening, setListening] = useState(false);

  function onKeyDown(e: React.KeyboardEvent) {
    if (!listening || MODIFIER_KEYS.includes(e.key)) return;
    e.preventDefault();
    if (e.key === "Escape") return setListening(false);
    const modifiers = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"].filter(Boolean);
    if (modifiers.length === 0) return; // a lone key would fire while typing anywhere
    // "KeyM" -> "M", "Digit1" -> "1"; others ("F9", "Space") are already what Rust expects.
    const key = e.code.replace(/^(Key|Digit)/, "");
    setListening(false);
    onChange([...modifiers, key].join("+"));
  }

  return (
    <button
      className={listening ? "on" : ""}
      onClick={() => setListening(true)}
      onKeyDown={onKeyDown}
      onBlur={() => setListening(false)}
    >
      {listening ? "Press keys…" : keys}
    </button>
  );
}

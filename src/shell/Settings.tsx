import { useEffect, useState } from "react";
import { commands, type ModuleId, type Motion, type Preset, type Settings } from "../bindings";
import { modules } from "../modules";

const PRESETS: { id: Preset; label: string }[] = [
  { id: "ocean", label: "Ocean" },
  { id: "graphite", label: "Graphite" },
];

const MOTIONS: { id: Motion; label: string }[] = [
  { id: "onOpen", label: "On open" },
  { id: "still", label: "Still" },
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
        <h2>Background</h2>
        <div className="segmented">
          {PRESETS.map((p) => (
            <button
              key={p.id}
              className={settings.preset === p.id ? "on" : ""}
              onClick={() => save({ ...settings, preset: p.id })}
            >
              {p.label}
            </button>
          ))}
        </div>
        <h2>Motion</h2>
        <div className="segmented">
          {MOTIONS.map((m) => (
            <button
              key={m.id}
              className={settings.motion === m.id ? "on" : ""}
              onClick={() => save({ ...settings, motion: m.id })}
            >
              {m.label}
            </button>
          ))}
        </div>
      </section>

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

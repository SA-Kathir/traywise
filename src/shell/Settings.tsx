import { useEffect, useState } from "react";
import { commands, type Accent, type Look, type Preset, type Settings } from "../bindings";
import { modules } from "../modules";

const LOOKS: { id: Look; label: string }[] = [
  { id: "dot", label: "Dot" },
  { id: "liquid", label: "Liquid" },
  { id: "windows", label: "Windows" },
];

const PRESETS: { id: Preset; label: string }[] = [
  { id: "ocean", label: "Ocean" },
  { id: "graphite", label: "Graphite" },
  { id: "forest", label: "Forest" },
  { id: "plum", label: "Plum" },
  { id: "ember", label: "Ember" },
  { id: "rose", label: "Rose" },
];

const ACCENTS: { id: Accent; label: string }[] = [
  { id: "red", label: "Red" },
  { id: "amber", label: "Amber" },
  { id: "violet", label: "Violet" },
  { id: "blue", label: "Blue" },
  { id: "green", label: "Green" },
];

// Segoe Fluent Icons glyph.
const CHEVRON = String.fromCodePoint(0xe76c);

type Props = { settings: Settings; onChange: (next: Settings) => void };

/** Saves settings: shows the change at once, and undoes it if Rust couldn't save it. */
export function useSave({ settings, onChange }: Props) {
  const [error, setError] = useState<string | null>(null);
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
  return { save, error, setError };
}

export function SettingsPage({ openModules, ...props }: Props & { openModules: () => void }) {
  const { settings } = props;
  const { save, error, setError } = useSave(props);
  // null until Windows has answered.
  const [autostart, setAutostart] = useState<boolean | null>(null);

  useEffect(() => {
    commands.getAutostart().then((r) => (r.status === "ok" ? setAutostart(r.data) : setError(r.error)));
  }, []);

  async function toggleAutostart(on: boolean) {
    const r = await commands.setAutostart(on);
    if (r.status === "ok") setAutostart(on);
    else setError(r.error);
  }

  return (
    <>
      <section>
        <button className="link-row" onClick={openModules}>
          <span className="grow">Modules</span>
          <span className="hint">
            {settings.enabledModules.length} of {Object.keys(modules).length} on
          </span>
          <span className="icon" aria-hidden>
            {CHEVRON}
          </span>
        </button>
      </section>

      <section>
        <h2>Look</h2>
        <div className="segmented" role="radiogroup" aria-label="Look">
          {LOOKS.map((l) => (
            <button
              key={l.id}
              role="radio"
              aria-checked={settings.look === l.id}
              className={settings.look === l.id ? "on" : ""}
              onClick={() => save({ ...settings, look: l.id })}
            >
              {l.label}
            </button>
          ))}
        </div>
        {settings.look === "dot" && (
          <>
            <div className="swatches" role="radiogroup" aria-label="Accent">
              {ACCENTS.map((a) => (
                <button
                  key={a.id}
                  role="radio"
                  aria-checked={settings.accent === a.id}
                  className={settings.accent === a.id ? "on" : ""}
                  onClick={() => save({ ...settings, accent: a.id })}
                >
                  {/* Its own accent class, so the swatch shows that colour. */}
                  <span className={`swatch dot-swatch accent-${a.id}`} aria-hidden />
                  {a.label}
                </button>
              ))}
            </div>
            <h2>Background</h2>
            <div className="segmented" role="radiogroup" aria-label="Background">
              {(
                [
                  ["Dots", false, false],
                  ["Plain", true, false],
                  ["Transparent", settings.plainBackground, true],
                ] as const
              ).map(([text, plainBackground, transparent]) => {
                const on = transparent ? settings.transparent : !settings.transparent && settings.plainBackground === plainBackground;
                return (
                  <button
                    key={text}
                    role="radio"
                    aria-checked={on}
                    className={on ? "on" : ""}
                    onClick={() => save({ ...settings, plainBackground, transparent })}
                  >
                    {text}
                  </button>
                );
              })}
            </div>
          </>
        )}
        {/* The Windows look uses Windows' own colours instead. */}
        {settings.look === "liquid" && (
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
        )}
        {settings.look === "liquid" && (
          <>
            <h2>Background</h2>
            <Choice
              label="Background"
              options={["Tinted", "Transparent"]}
              value={settings.transparent}
              onChange={(transparent) => save({ ...settings, transparent })}
            />
            <h2>Tiles</h2>
            <Choice
              label="Tiles"
              options={["Shaded", "Single hue"]}
              value={settings.singleHue}
              onChange={(singleHue) => save({ ...settings, singleHue })}
            />
          </>
        )}
      </section>

      <section>
        <h2>Shortcuts</h2>
        <div className="row">
          <span className="grow">Open or close Traywise</span>
          <ShortcutPicker keys={settings.panelShortcut} onChange={(panelShortcut) => save({ ...settings, panelShortcut })} />
        </div>
        {settings.enabledModules.includes("audio-devices") && (
          <div className="row">
            <span className="grow">Mute all microphones</span>
            <ShortcutPicker keys={settings.micShortcut} onChange={(micShortcut) => save({ ...settings, micShortcut })} />
          </div>
        )}
      </section>

      <section>
        <h2>Opening</h2>
        <label className="row">
          <span className="grow">Open from the right edge</span>
          <input type="checkbox" checked={settings.edgeOpen} onChange={(e) => save({ ...settings, edgeOpen: e.target.checked })} />
        </label>
        <label className="row">
          <span className="grow">Show a handle on the edge</span>
          <input
            type="checkbox"
            disabled={!settings.edgeOpen}
            checked={settings.edgeHandle}
            onChange={(e) => save({ ...settings, edgeHandle: e.target.checked })}
          />
        </label>
        <p className="hint">Rest the mouse on the middle of the right edge for a moment. It closes when the mouse leaves.</p>
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
        <label className="row">
          <span className="grow">Check for updates</span>
          <input
            type="checkbox"
            checked={settings.checkUpdates}
            onChange={(e) => save({ ...settings, checkUpdates: e.target.checked })}
          />
        </label>
      </section>

      {error && <p className="error">{error}</p>}
    </>
  );
}

/** A two-way choice saved as a yes/no setting: the second option means yes. */
function Choice(props: { label: string; options: [string, string]; value: boolean; onChange: (value: boolean) => void }) {
  return (
    <div className="segmented" role="radiogroup" aria-label={props.label}>
      {props.options.map((text, i) => {
        const on = props.value === (i === 1);
        return (
          <button key={text} role="radio" aria-checked={on} className={on ? "on" : ""} onClick={() => props.onChange(i === 1)}>
            {text}
          </button>
        );
      })}
    </div>
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

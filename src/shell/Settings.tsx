import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useState, type ReactNode } from "react";
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
  const [version, setVersion] = useState("");

  useEffect(() => {
    commands.getAutostart().then((r) => (r.status === "ok" ? setAutostart(r.data) : setError(r.error)));
    getVersion().then(setVersion);
  }, []);

  async function toggleAutostart(on: boolean) {
    const r = await commands.setAutostart(on);
    if (r.status === "ok") setAutostart(on);
    else setError(r.error);
  }

  const dotBackground = settings.transparent ? "transparent" : settings.plainBackground ? "plain" : "dots";

  return (
    <>
      <section>
        <button className="setting link" onClick={openModules}>
          <span className="setting-text">
            <span className="setting-label">Modules</span>
            <span className="setting-desc">
              {settings.enabledModules.length} of {Object.keys(modules).length} on
            </span>
          </span>
          <span className="icon" aria-hidden>
            {CHEVRON}
          </span>
        </button>
      </section>

      <section>
        <h2>Appearance</h2>
        <Row label="Look" stacked>
          <Pick label="Look" options={LOOKS} value={settings.look} onChange={(look) => save({ ...settings, look })} />
        </Row>
        {settings.look === "dot" && (
          <>
            <Row label="Accent">
              <div className="colour-pick" role="radiogroup" aria-label="Accent">
                {ACCENTS.map((a) => (
                  <button
                    key={a.id}
                    role="radio"
                    aria-checked={settings.accent === a.id}
                    aria-label={a.label}
                    title={a.label}
                    className={settings.accent === a.id ? "on" : ""}
                    onClick={() => save({ ...settings, accent: a.id })}
                  >
                    {/* Its own accent class, so the dot shows that colour. */}
                    <span className={`swatch dot-swatch accent-${a.id}`} aria-hidden />
                  </button>
                ))}
              </div>
            </Row>
            <Row label="Background" stacked>
              <Pick
                label="Background"
                options={[
                  { id: "dots", label: "Dots" },
                  { id: "plain", label: "Plain" },
                  { id: "transparent", label: "Transparent" },
                ]}
                value={dotBackground}
                onChange={(b) =>
                  save({ ...settings, transparent: b === "transparent", plainBackground: b === "transparent" ? settings.plainBackground : b === "plain" })
                }
              />
            </Row>
          </>
        )}
        {settings.look === "liquid" && (
          <>
            <Row label="Colour">
              <div className="colour-pick" role="radiogroup" aria-label="Colour">
                {PRESETS.map((p) => (
                  <button
                    key={p.id}
                    role="radio"
                    aria-checked={settings.preset === p.id}
                    aria-label={p.label}
                    title={p.label}
                    className={settings.preset === p.id ? "on" : ""}
                    onClick={() => save({ ...settings, preset: p.id })}
                  >
                    {/* Its own preset class, so the swatch shows that preset's colours. */}
                    <span className={`swatch bg-${p.id}`} aria-hidden />
                  </button>
                ))}
              </div>
            </Row>
            <Row label="Background" stacked>
              <Choice
                label="Background"
                options={["Tinted", "Transparent"]}
                value={settings.transparent}
                onChange={(transparent) => save({ ...settings, transparent })}
              />
            </Row>
            <Row label="Tiles" stacked>
              <Choice label="Tiles" options={["Shaded", "Single hue"]} value={settings.singleHue} onChange={(singleHue) => save({ ...settings, singleHue })} />
            </Row>
          </>
        )}
        {settings.look === "windows" && <p className="setting-desc">Follows Windows' light or dark mode and its colours.</p>}
      </section>

      <section>
        <h2>Opening</h2>
        <Row label="Shortcut" desc="Opens or closes Traywise">
          <ShortcutPicker keys={settings.panelShortcut} onChange={(panelShortcut) => save({ ...settings, panelShortcut })} />
        </Row>
        <SwitchRow
          label="Open from the right edge"
          desc="Rest the mouse on its middle"
          checked={settings.edgeOpen}
          onChange={(edgeOpen) => save({ ...settings, edgeOpen })}
        />
        {settings.edgeOpen && (
          <SwitchRow
            label="Show a handle"
            desc="A small mark on the edge, where to rest the mouse"
            checked={settings.edgeHandle}
            onChange={(edgeHandle) => save({ ...settings, edgeHandle })}
          />
        )}
      </section>

      <section>
        <h2>System</h2>
        <SwitchRow label="Start with Windows" checked={autostart ?? false} disabled={autostart === null} onChange={toggleAutostart} />
        <SwitchRow
          label="Check for updates"
          desc="About once a day, from GitHub"
          checked={settings.checkUpdates}
          onChange={(checkUpdates) => save({ ...settings, checkUpdates })}
        />
        <Row label="Traywise" desc="Free and open source, under the GPL">
          <span className="setting-value">{version}</span>
        </Row>
      </section>

      {error && <p className="error">{error}</p>}
    </>
  );
}

/** A setting: its name (and a line on what it does), then its control; `stacked` puts the
 *  control under the name, for choices that need the width. */
export function Row(props: { label: string; desc?: string; stacked?: boolean; children: ReactNode }) {
  return (
    <div className={props.stacked ? "setting stacked" : "setting"}>
      <span className="setting-text">
        <span className="setting-label">{props.label}</span>
        {props.desc && <span className="setting-desc">{props.desc}</span>}
      </span>
      {props.children}
    </div>
  );
}

/** A setting that's on or off; the whole row toggles it. */
export function SwitchRow(props: { label: string; desc?: string; checked: boolean; disabled?: boolean; onChange: (on: boolean) => void }) {
  return (
    <label className="setting">
      <span className="setting-text">
        <span className="setting-label">{props.label}</span>
        {props.desc && <span className="setting-desc">{props.desc}</span>}
      </span>
      <input type="checkbox" checked={props.checked} disabled={props.disabled} onChange={(e) => props.onChange(e.target.checked)} />
    </label>
  );
}

/** One of a few choices, as a segmented control. */
function Pick<T extends string>(props: { label: string; options: { id: T; label: string }[]; value: T; onChange: (value: T) => void }) {
  return (
    <div className="segmented" role="radiogroup" aria-label={props.label}>
      {props.options.map((o) => (
        <button key={o.id} role="radio" aria-checked={props.value === o.id} className={props.value === o.id ? "on" : ""} onClick={() => props.onChange(o.id)}>
          {o.label}
        </button>
      ))}
    </div>
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
export function ShortcutPicker({ keys, onChange }: { keys: string; onChange: (keys: string) => void }) {
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

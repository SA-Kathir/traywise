import { useState } from "react";
import { commands, type FocusStyle, type Settings } from "../../bindings";
import { Pick, Row, SwitchRow } from "../../shell/Settings";
import { useSettings } from "../../shell/SettingsContext";

const MAX_PRESETS = 4;
const valid = (n: number) => Number.isInteger(n) && n >= 1 && n <= 480;

/** A short centred number box (minutes). */
function Minutes({ label, value, onChange }: { label: string; value: number; onChange: (n: number) => void }) {
  return (
    <input
      className="text timer-minutes"
      type="number"
      min={1}
      max={480}
      aria-label={label}
      value={value || ""}
      onChange={(e) => onChange(Number(e.target.value))}
    />
  );
}

export default function Panel() {
  const { settings, save } = useSettings();
  const set = (patch: Partial<Settings>) => save({ ...settings, ...patch });
  const [focus, setFocus] = useState(45);
  const [rest, setRest] = useState(10);
  const presets = settings.focusPresets;
  return (
    <>
      <Row label="Tile style" desc="How the Focus tile draws the countdown">
        <Pick<FocusStyle>
          label="Tile style"
          options={[
            { id: "arc", label: "Arc" },
            { id: "hourglass", label: "Hourglass" },
          ]}
          value={settings.focusStyle}
          onChange={(focusStyle) => set({ focusStyle })}
        />
      </Row>

      {presets.map((p, i) => (
        <Row key={`${p.focus}/${p.rest}/${i}`} label={`${p.focus} focus · ${p.rest} rest`} desc={i === 0 ? "Default: what tapping the tile starts" : undefined}>
          <button onClick={() => commands.focusStart(p.focus, p.rest)}>Start</button>
          <button aria-label="Remove this length" disabled={presets.length === 1} onClick={() => set({ focusPresets: presets.filter((_, j) => j !== i) })}>
            ×
          </button>
        </Row>
      ))}
      {presets.length < MAX_PRESETS && (
        <Row label="Add a length" desc="Focus and rest, in minutes">
          <Minutes label="Focus minutes" value={focus} onChange={setFocus} />
          <Minutes label="Rest minutes" value={rest} onChange={setRest} />
          <button disabled={!valid(focus) || !valid(rest)} onClick={() => set({ focusPresets: [...presets, { focus, rest }] })}>
            Add
          </button>
        </Row>
      )}

      <Row label="Long rest" desc="After the fourth round, in minutes">
        <Minutes label="Long rest minutes" value={settings.focusLongRest} onChange={(n) => valid(n) && set({ focusLongRest: n })} />
      </Row>
      <SwitchRow label="Start each phase by itself" checked={settings.focusAutoNext} onChange={(focusAutoNext) => set({ focusAutoNext })} />
      <SwitchRow label="Do not disturb while focusing" checked={settings.focusDndFocus} onChange={(focusDndFocus) => set({ focusDndFocus })} />
      <SwitchRow label="Do not disturb during rests" checked={settings.focusDndRest} onChange={(focusDndRest) => set({ focusDndRest })} />
      <SwitchRow label="Play a sound when a phase ends" checked={settings.focusChime} onChange={(focusChime) => set({ focusChime })} />
    </>
  );
}

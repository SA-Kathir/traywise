import { useState } from "react";
import { commands, type FocusStyle, type Settings } from "../../bindings";
import { Collapsible, Pick, Row, SwitchRow } from "../../shell/Settings";
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

type Section = "lengths" | "dnd" | "auto" | "look" | "sound";

export default function Panel() {
  const { settings, save, showTile } = useSettings();
  const set = (patch: Partial<Settings>) => save({ ...settings, ...patch });
  const [focus, setFocus] = useState(45);
  const [rest, setRest] = useState(10);
  const [open, setOpen] = useState<Section | null>(null);
  const toggle = (s: Section) => () => setOpen(open === s ? null : s);
  const presets = settings.focusPresets;
  // Starting takes you Home to watch the tile, if the board has one; otherwise you stay here.
  const start = (f: number, r: number) => commands.focusStart(f, r).then(() => showTile("focus/focus"));
  return (
    <>
      <Row label="Start">
        {presets.map((p, i) => (
          <button key={`${p.focus}/${p.rest}/${i}`} title={`${p.focus} focus, ${p.rest} rest`} onClick={() => start(p.focus, p.rest)}>
            {p.focus}/{p.rest}
          </button>
        ))}
      </Row>
      <Row label="Custom">
        <Minutes label="Focus minutes" value={focus} onChange={setFocus} />
        <Minutes label="Rest minutes" value={rest} onChange={setRest} />
        <button disabled={!valid(focus) || !valid(rest)} onClick={() => start(focus, rest)}>
          Start
        </button>
      </Row>

      <Collapsible title="Lengths" summary={`${presets.map((p) => `${p.focus}/${p.rest}`).join(", ")}, long rest ${settings.focusLongRest}`} open={open === "lengths"} onToggle={toggle("lengths")}>
        {presets.map((p, i) => (
          <Row key={`${p.focus}/${p.rest}/${i}`} label={`${p.focus} focus Â· ${p.rest} rest`} desc={i === 0 ? "Default: what the tile starts" : undefined}>
            <button aria-label="Remove this length" disabled={presets.length === 1} onClick={() => set({ focusPresets: presets.filter((_, j) => j !== i) })}>
              ×
            </button>
          </Row>
        ))}
        {presets.length < MAX_PRESETS && (
          <Row label="Save the custom length">
            <button disabled={!valid(focus) || !valid(rest)} onClick={() => set({ focusPresets: [...presets, { focus, rest }] })}>
              Save {focus}/{rest}
            </button>
          </Row>
        )}
        <Row label="Long rest" desc="After the fourth round, in minutes">
          <Minutes label="Long rest minutes" value={settings.focusLongRest} onChange={(n) => valid(n) && set({ focusLongRest: n })} />
        </Row>
      </Collapsible>

      <Collapsible
        title="Do not disturb"
        summary={settings.focusDndFocus ? (settings.focusDndRest ? "During focus and rests" : "During focus") : settings.focusDndRest ? "During rests" : "Off"}
        open={open === "dnd"}
        onToggle={toggle("dnd")}
      >
        <SwitchRow label="While focusing" checked={settings.focusDndFocus} onChange={(focusDndFocus) => set({ focusDndFocus })} />
        <SwitchRow label="During rests" checked={settings.focusDndRest} onChange={(focusDndRest) => set({ focusDndRest })} />
      </Collapsible>

      <Collapsible title="Automation" summary={settings.focusAutoNext ? "Phases start by themselves" : "Wait for a tap"} open={open === "auto"} onToggle={toggle("auto")}>
        <SwitchRow label="Start each phase by itself" checked={settings.focusAutoNext} onChange={(focusAutoNext) => set({ focusAutoNext })} />
      </Collapsible>

      <Collapsible title="Look" summary={settings.focusStyle === "arc" ? "Arc" : "Hourglass"} open={open === "look"} onToggle={toggle("look")}>
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
      </Collapsible>

      <Collapsible title="Sound" summary={settings.focusChime ? "Chime at each phase end" : "Silent"} open={open === "sound"} onToggle={toggle("sound")}>
        <SwitchRow label="Play a sound when a phase ends" checked={settings.focusChime} onChange={(focusChime) => set({ focusChime })} />
      </Collapsible>
    </>
  );
}

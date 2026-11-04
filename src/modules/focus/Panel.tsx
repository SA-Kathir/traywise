import { useState } from "react";
import { commands } from "../../bindings";
import { SwitchRow } from "../../shell/Settings";
import { useSettings } from "../../shell/SettingsContext";

export default function Panel() {
  const { settings, save } = useSettings();
  const [minutes, setMinutes] = useState(10);
  const valid = Number.isInteger(minutes) && minutes >= 1 && minutes <= 480;
  return (
    <>
      <label className="row">
        <span className="grow">Custom timer (minutes, 1 to 480)</span>
        <input type="number" min={1} max={480} value={minutes || ""} onChange={(e) => setMinutes(Number(e.target.value))} style={{ width: "4.5em" }} />
        <button disabled={!valid} onClick={() => commands.focusSet({ minutes })}>
          Start
        </button>
      </label>
      <SwitchRow label="Play a sound when time's up" checked={settings.focusChime} onChange={(focusChime) => save({ ...settings, focusChime })} />
    </>
  );
}

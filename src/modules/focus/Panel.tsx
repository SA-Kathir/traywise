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
        <span className="grow">Timer in minutes</span>
        <input
          className="text timer-minutes"
          type="number"
          min={1}
          max={480}
          aria-label="Minutes"
          value={minutes || ""}
          onChange={(e) => setMinutes(Number(e.target.value))}
          onKeyDown={(e) => e.key === "Enter" && valid && commands.focusSet({ minutes })}
        />
        <button disabled={!valid} onClick={() => commands.focusSet({ minutes })}>
          Start
        </button>
      </label>
      <SwitchRow label="Play a sound when time's up" checked={settings.focusChime} onChange={(focusChime) => save({ ...settings, focusChime })} />
    </>
  );
}

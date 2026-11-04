import { SwitchRow } from "../../shell/Settings";
import { useSettings } from "../../shell/SettingsContext";

export default function Panel() {
  const { settings, save } = useSettings();
  return <SwitchRow label="Play a sound when time's up" checked={settings.focusChime} onChange={(focusChime) => save({ ...settings, focusChime })} />;
}

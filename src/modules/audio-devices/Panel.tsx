import { useEffect, useState } from "react";
import { commands, events, type AudioDevicesState } from "../../bindings";

export default function Panel() {
  const [state, setState] = useState<AudioDevicesState | null>(null);

  useEffect(() => {
    let live = false; // an event beat the first fetch: don't overwrite it with older data
    const unlisten = events.audioDevicesChanged.listen((e) => {
      live = true;
      setState(e.payload);
    });
    commands.audioDevicesState().then((s) => live || setState(s));
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);

  if (!state) return null;
  if (state.outputs.length === 0) return <p className="hint">No output device.</p>;

  return (
    <div className="segmented vertical" role="radiogroup" aria-label="Output device">
      {state.outputs.map((d) => (
        <button
          key={d.id}
          role="radio"
          aria-checked={d.isDefault}
          className={d.isDefault ? "on" : ""}
          title={d.name}
          onClick={() => d.isDefault || commands.audioDevicesSetOutput(d.id)}
        >
          {d.name}
        </button>
      ))}
    </div>
  );
}

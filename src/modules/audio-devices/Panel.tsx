import { useEffect, useState } from "react";
import { commands, events, type AudioDevicesState } from "../../bindings";

// Segoe Fluent Icons glyphs.
const MIC = String.fromCodePoint(0xe720);
const MIC_OFF = String.fromCodePoint(0xec54);

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

  return (
    <>
      <Outputs state={state} />
      <Mics state={state} />
    </>
  );
}

function Outputs({ state }: { state: AudioDevicesState }) {
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

function Mics({ state: { micsMuted, shortcut } }: { state: AudioDevicesState }) {
  if (micsMuted === null) return <p className="hint">No microphone.</p>;
  return (
    <>
      <div className="row mics">
        <span className="icon" aria-hidden>
          {micsMuted ? MIC_OFF : MIC}
        </span>
        <span className="grow">{micsMuted ? "All microphones muted" : "Microphones on"}</span>
        <button className={micsMuted ? "on" : ""} onClick={() => commands.audioDevicesMuteMics(!micsMuted)}>
          {micsMuted ? "Unmute all" : "Mute all"}
        </button>
      </div>
      {shortcut &&
        (shortcut.error ? (
          <p className="error" title={shortcut.error}>
            {shortcut.keys} is already used by another app. Pick other keys in Settings › Shortcuts.
          </p>
        ) : (
          <p className="hint">Shortcut: {shortcut.keys}</p>
        ))}
    </>
  );
}

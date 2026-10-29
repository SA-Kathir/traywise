import { commands, events, type AudioDevicesState } from "../../bindings";
import { Tile, TileCard } from "../../shell/Tile";
import { useLive } from "../../shell/useLive";
import type { TileProps } from "..";

// Segoe Fluent Icons glyphs.
const MIC = String.fromCodePoint(0xe720);
const MIC_OFF = String.fromCodePoint(0xec54);
const HEADPHONES = String.fromCodePoint(0xe7f6);
const SPEAKERS = String.fromCodePoint(0xe7f5);

const useAudioDevices = () => useLive(commands.audioDevicesState, events.audioDevicesChanged);

/** "Headphones (Realtek(R) Audio)" -> "Headphones": the part in brackets is the sound card. */
const shortName = (name: string) => name.replace(/\s*\(.*\)\s*$/, "") || name;

/** A guess from the name: Windows doesn't say whether an output is headphones. */
const outputIcon = (name: string) => (/headphone|headset|earbud/i.test(name) ? HEADPHONES : SPEAKERS);

/** Home tiles: the output device (opens the list; at 2×2, the list itself) and mute-all for microphones. */
export function Tiles({ kind, size, open }: TileProps) {
  const state = useAudioDevices();
  if (!state) return null;
  if (kind === "mics") {
    const muted = state.micsMuted;
    return (
      <Tile
        size={size}
        icon={muted ? MIC_OFF : MIC}
        label="Microphones"
        detail={muted === null ? "None" : muted ? "All muted" : "On"}
        on={muted === true}
        onToggle={muted === null ? undefined : () => commands.audioDevicesMuteMics(!muted)}
        onOpen={open}
      />
    );
  }
  const output = state.outputs.find((d) => d.isDefault);
  if (size === "2x2")
    return (
      <TileCard size={size} label="Output" onOpen={open}>
        <Outputs state={state} />
      </TileCard>
    );
  return (
    <Tile
      size={size}
      icon={output ? outputIcon(output.name) : SPEAKERS}
      label="Output"
      detail={output ? shortName(output.name) : "No device"}
      onOpen={open}
    />
  );
}

export default function Panel() {
  const state = useAudioDevices();
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
  const shorts = state.outputs.map((d) => shortName(d.name));
  return (
    <div className="segmented vertical" role="radiogroup" aria-label="Output device">
      {state.outputs.map((d, i) => (
        <button
          key={d.id}
          role="radio"
          aria-checked={d.isDefault}
          className={d.isDefault ? "on" : ""}
          title={d.name}
          onClick={() => d.isDefault || commands.audioDevicesSetOutput(d.id)}
        >
          <span className="icon" aria-hidden>
            {outputIcon(d.name)}
          </span>
          {/* Two "Speakers" on different sound cards keep their full names, to tell them apart. */}
          {shorts.indexOf(shorts[i]) === shorts.lastIndexOf(shorts[i]) ? shorts[i] : d.name}
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

import { useEffect, useRef, useState } from "react";
import { commands, events, type MixerState, type Target } from "../../bindings";

// Segoe Fluent Icons glyphs.
const SPEAKER = String.fromCodePoint(0xe767);
const MUTED = String.fromCodePoint(0xe74f);

/** At most one volume change per frame (~60 a second) while a slider is dragged. */
const SEND_EVERY_MS = 16;

export default function Panel() {
  const [state, setState] = useState<MixerState | null>(null);

  useEffect(() => {
    let live = false; // an event beat the first fetch: don't overwrite it with older data
    const unlisten = events.mixerChanged.listen((e) => {
      live = true;
      setState(e.payload);
    });
    commands.mixerState().then((s) => live || setState(s));
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);

  if (!state) return null;
  if (!state.master) return <p className="hint">No output device.</p>;

  return (
    <>
      <Row name="Master" volume={state.master.volume} muted={state.master.muted} target="master" />
      {state.apps.map((app) => (
        <Row
          key={app.pid}
          name={app.name}
          icon={app.icon}
          volume={app.volume}
          muted={app.muted}
          target={{ app: app.pid }}
        />
      ))}
      {state.apps.length === 0 && <p className="hint">No apps are playing audio.</p>}
    </>
  );
}

type RowProps = { name: string; icon?: string | null; volume: number; muted: boolean; target: Target };

function Row({ name, icon, volume, muted, target }: RowProps) {
  return (
    <div className="mixer-row">
      <button
        className="icon-button"
        aria-label={muted ? `Unmute ${name}` : `Mute ${name}`}
        title={muted ? "Unmute" : "Mute"}
        onClick={() => commands.mixerSetMute(target, !muted)}
      >
        {muted ? MUTED : SPEAKER}
      </button>
      {icon ? <img className="app-icon" src={icon} alt="" /> : <span className="app-icon" />}
      <span className="mixer-name" title={name}>
        {name}
      </span>
      <VolumeSlider name={name} value={volume} onChange={(v) => commands.mixerSetVolume(target, v)} />
    </div>
  );
}

function VolumeSlider({ name, value, onChange }: { name: string; value: number; onChange: (v: number) => void }) {
  // While dragging, show the thumb where the pointer is, not the slightly older value Windows echoes back.
  const [dragged, setDragged] = useState<number | null>(null);
  const dragging = useRef(false);
  const lastSent = useRef(0);
  const pending = useRef<number | null>(null);
  const timer = useRef<number | undefined>(undefined);

  useEffect(() => () => clearTimeout(timer.current), []);

  function send(v: number) {
    const wait = SEND_EVERY_MS - (performance.now() - lastSent.current);
    if (wait <= 0) {
      lastSent.current = performance.now();
      onChange(v);
      return;
    }
    // Too soon: send the latest value once the frame is up, so the final position always lands.
    pending.current = v;
    timer.current ??= window.setTimeout(() => {
      timer.current = undefined;
      lastSent.current = performance.now();
      if (pending.current !== null) onChange(pending.current);
    }, wait);
  }

  const shown = dragged ?? value;
  const endDrag = () => {
    dragging.current = false;
    setDragged(null);
  };
  return (
    <>
      <input
        type="range"
        min={0}
        max={100}
        value={shown}
        aria-label={`${name} volume`}
        onPointerDown={() => (dragging.current = true)}
        onPointerUp={endDrag}
        onPointerCancel={endDrag}
        onChange={(e) => {
          const v = Number(e.target.value);
          if (dragging.current) setDragged(v);
          send(v);
        }}
      />
      <span className="mixer-value">{shown}</span>
    </>
  );
}

import { useEffect, useRef, useState, type CSSProperties } from "react";
import { commands, events, type Target } from "../../bindings";
import { Tile, TileCard } from "../../shell/Tile";
import { useLive } from "../../shell/useLive";
import type { Size, TileProps } from "..";

// Segoe Fluent Icons glyphs.
const SPEAKER = String.fromCodePoint(0xe767);
const MUTED = String.fromCodePoint(0xe74f);
// Stand-ins where there's no app icon: the output device, Windows' sounds, any other app.
const DEVICE = String.fromCodePoint(0xe7f5);
const BELL = String.fromCodePoint(0xea8f);
const APP = String.fromCodePoint(0xecaa);

/** At most one volume change per frame (~60 a second) while a slider is dragged. */
const SEND_EVERY_MS = 16;

const useMixer = () => useLive(commands.mixerState, events.mixerChanged);

/** How many apps each size has room for, under the master volume. */
const APPS_SHOWN: Partial<Record<Size, number>> = { "2x2": 2, "4x2": 3 };

/** Home tile: mute (1×1), the master volume, and at 2×2 and 4×2 the apps playing. */
export function Tiles({ size, open }: TileProps) {
  const state = useMixer();
  if (!state) return null;
  const master = state.master;
  if (size === "1x1")
    return (
      <Tile
        size={size}
        icon={master?.muted ? MUTED : SPEAKER}
        label="Volume"
        detail={master ? String(master.volume) : "No device"}
        on={master?.muted}
        level={master && !master.muted ? master.volume : undefined}
        onToggle={master ? () => commands.mixerSetMute("master", !master.muted) : open}
      />
    );
  // Apps first, Windows' own sounds last.
  const apps = state.apps.filter((a) => !a.systemSounds);
  return (
    <TileCard size={size} label="Volume" detail={apps.length === 0 ? "None playing" : `${apps.length} playing`} onOpen={open}>
      {master ? (
        <Row name="Master" glyph={DEVICE} volume={master.volume} muted={master.muted} target="master" />
      ) : (
        <p className="hint">No output device.</p>
      )}
      {apps.slice(0, APPS_SHOWN[size] ?? 0).map((app) => (
        <Row
          key={app.pid}
          name={app.name}
          icon={app.icon}
          glyph={APP}
          volume={app.volume}
          muted={app.muted}
          target={{ app: app.pid }}
        />
      ))}
    </TileCard>
  );
}

export default function Panel() {
  const state = useMixer();
  if (!state) return null;
  if (!state.master) return <p className="hint">No output device.</p>;

  return (
    <>
      <Row name="Master" glyph={DEVICE} volume={state.master.volume} muted={state.master.muted} target="master" />
      {state.apps.map((app) => (
        <Row
          key={app.pid}
          name={app.name}
          icon={app.icon}
          glyph={app.systemSounds ? BELL : APP}
          volume={app.volume}
          muted={app.muted}
          target={{ app: app.pid }}
        />
      ))}
      {state.apps.length === 0 && <p className="hint">No apps are playing audio.</p>}
    </>
  );
}

type RowProps = {
  name: string;
  /** PNG data URL; `glyph` is shown when there is none. */
  icon?: string | null;
  glyph: string;
  volume: number;
  muted: boolean;
  target: Target;
};

function Row({ name, icon, glyph, volume, muted, target }: RowProps) {
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
      {icon ? (
        <img className="app-icon" src={icon} alt="" />
      ) : (
        <span className="app-icon glyph" aria-hidden>
          {glyph}
        </span>
      )}
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
        style={{ "--fill": `${shown}%` } as CSSProperties}
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

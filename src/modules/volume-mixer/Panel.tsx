import { useEffect, useRef, useState, type CSSProperties } from "react";
import { commands, events, type Target } from "../../bindings";
import { Num, Tile } from "../../shell/Tile";
import { useLive } from "../../shell/useLive";
import type { TileProps } from "..";

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

/** "Headphones (Realtek(R) Audio)" -> "Headphones": the part in brackets is the sound card. */
const shortName = (name: string) => name.replace(/\s*\(.*\)\s*$/, "") || name;

/** Home tile, as in the design: the master volume as a number and a bar (1×1: the whole tile
 *  fills to it), the output device's name from 4×1 up, and at 4×2 the apps playing beside it. */
export function Tiles({ size, open }: TileProps) {
  const state = useMixer();
  if (!state) return null;
  const master = state.master;
  if (!master)
    return <Tile size={size} icon={MUTED} label="Volume" detail="No device" onOpen={open} />;
  const level = master.muted ? 0 : master.volume;
  const mute = () => commands.mixerSetMute("master", !master.muted);
  if (size === "1x1")
    return (
      <button
        className="tile vol size-1x1"
        style={{ "--level": `${level}%` } as CSSProperties}
        title={master.muted ? "Volume: muted (click to unmute)" : `Volume: ${master.volume} (click to mute)`}
        onClick={mute}
      >
        <Num text={String(level)} />
        <span className="vol-icon" aria-hidden>
          {master.muted ? MUTED : SPEAKER}
        </span>
      </button>
    );
  const device = state.device && shortName(state.device);
  const big = size === "2x2" || size === "4x2";
  const head = (
    <>
      <button className="tile-title" title="Details" onClick={open}>
        <span className="tile-label">Volume</span>
        {size === "4x1" && device && <span className="tile-detail">{device}</span>}
      </button>
      {!big && <Num text={String(level)} />}
    </>
  );
  const main = (
    <div className="vol-main">
      <div className="vol-head">{head}</div>
      {big && (
        <>
          <span className="vol-big">
            <Num text={String(level)} />
          </span>
          {device && <span className="tile-detail">{device}</span>}
        </>
      )}
      <div className="vol-bar">
        <VolumeSlider name="Master" value={level} onChange={(v) => commands.mixerSetVolume("master", v)} bare />
        <button className="vol-icon" aria-label={master.muted ? "Unmute" : "Mute"} title={master.muted ? "Unmute" : "Mute"} onClick={mute}>
          {master.muted ? MUTED : SPEAKER}
        </button>
      </div>
    </div>
  );
  if (size !== "4x2") return <div className={`tile vol size-${size}`}>{main}</div>;
  // Apps first, Windows' own sounds last; three fit.
  const apps = state.apps.filter((a) => !a.systemSounds).slice(0, 3);
  return (
    <div className="tile vol size-4x2">
      {main}
      <div className="vol-apps">
        {apps.length === 0 && <p className="hint">No apps playing.</p>}
        {apps.map((app) => (
          <div key={app.pid} className="vol-app">
            <div className="vol-head">
              <span className="tile-detail" title={app.name}>
                {app.name}
              </span>
              <Num text={String(app.muted ? 0 : app.volume)} />
            </div>
            <VolumeSlider name={app.name} value={app.volume} onChange={(v) => commands.mixerSetVolume({ app: app.pid }, v)} bare />
          </div>
        ))}
      </div>
    </div>
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

/** `bare`: the slider alone, without its number. */
function VolumeSlider({ name, value, onChange, bare }: { name: string; value: number; onChange: (v: number) => void; bare?: boolean }) {
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
      {!bare && <span className="mixer-value">{shown}</span>}
    </>
  );
}

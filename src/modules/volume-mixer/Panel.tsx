import { Fragment, useState, type CSSProperties } from "react";
import { commands, events, type AppAudio, type OutputDevice, type Target } from "../../bindings";
import { Num, Tile } from "../../shell/Tile";
import { Slider as VolumeSlider } from "../../shell/Slider";
import { useLive } from "../../shell/useLive";
import type { TileProps } from "..";

// Segoe Fluent Icons glyphs.
const SPEAKER = String.fromCodePoint(0xe767);
const MUTED = String.fromCodePoint(0xe74f);
// Stand-ins where there's no app icon: the output device, Windows' sounds, any other app.
const DEVICE = String.fromCodePoint(0xe7f5);
const BELL = String.fromCodePoint(0xea8f);
const APP = String.fromCodePoint(0xecaa);
const CHEVRON = String.fromCodePoint(0xe70d);
const CHECK = String.fromCodePoint(0xe73e);

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
  // The app whose output list is open.
  const [picking, setPicking] = useState<number | null>(null);
  if (!state) return null;
  if (!state.master) return <p className="hint">No output device.</p>;
  // Empty where Windows doesn't support it; with one device there's nothing to pick.
  const canRoute = state.outputs.length > 1;
  const moved = state.apps.filter((a) => a.output).length;

  return (
    <>
      <Row name="Master" glyph={DEVICE} volume={state.master.volume} muted={state.master.muted} target="master" />
      {state.apps.map((app) => (
        <Fragment key={app.pid}>
          <Row
            name={app.name}
            icon={app.icon}
            glyph={app.systemSounds ? BELL : APP}
            volume={app.volume}
            muted={app.muted}
            target={{ app: app.pid }}
            output={canRoute && !app.systemSounds ? outputName(app, state.outputs) : undefined}
            moved={app.output !== null}
            onOutput={() => setPicking(picking === app.pid ? null : app.pid)}
          />
          {canRoute && picking === app.pid && <OutputPicker app={app} outputs={state.outputs} onPicked={() => setPicking(null)} />}
        </Fragment>
      ))}
      {state.apps.length === 0 && <p className="hint">No apps are playing audio.</p>}
      {canRoute && moved > 0 && (
        <div className="row mixer-reset">
          <span className="grow hint">{moved === 1 ? "1 app" : `${moved} apps`} not on Default</span>
          <button onClick={() => commands.mixerResetOutputs()}>Reset all</button>
        </div>
      )}
    </>
  );
}

/** "Default", or the short name of the device the app is set to. */
const outputName = (app: AppAudio, outputs: OutputDevice[]) => {
  const device = app.output && outputs.find((d) => d.id === app.output);
  return device ? shortName(device.name) : "Default";
};

function OutputPicker({ app, outputs, onPicked }: { app: AppAudio; outputs: OutputDevice[]; onPicked: () => void }) {
  const pick = (id: string | null) => {
    if (id !== app.output) commands.mixerSetOutput(app.pid, id);
    onPicked();
  };
  const current = outputs.find((d) => d.isDefault);
  const choices = [{ id: null, name: current ? `Default (${shortName(current.name)})` : "Default" }, ...outputs];
  return (
    <div className="segmented vertical mixer-outputs" role="radiogroup" aria-label={`${app.name} output`}>
      {choices.map((d) => (
        <button
          key={d.id ?? "default"}
          role="radio"
          aria-checked={d.id === app.output}
          className={d.id === app.output ? "on" : ""}
          title={d.name}
          onClick={() => pick(d.id)}
        >
          {d.id === app.output && (
            <span className="check" aria-hidden>
              {CHECK}
            </span>
          )}
          {d.name}
        </button>
      ))}
    </div>
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
  /** The app's output device, shown under its name; clicking it calls `onOutput`. */
  output?: string;
  /** Set to a device rather than following the default: the output shows in the accent colour. */
  moved?: boolean;
  onOutput?: () => void;
};

function Row({ name, icon, glyph, volume, muted, target, output, moved, onOutput }: RowProps) {
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
      <span className="mixer-name">
        <span className="mixer-title" title={name}>
          {name}
        </span>
        {output && (
          <button className={moved ? "mixer-output moved" : "mixer-output"} title="Output device" onClick={onOutput}>
            <span className="glyph" aria-hidden>
              {SPEAKER}
            </span>
            <span className="mixer-output-name">{output}</span>
            <span className="glyph" aria-hidden>
              {CHEVRON}
            </span>
          </button>
        )}
      </span>
      <VolumeSlider name={name} value={volume} onChange={(v) => commands.mixerSetVolume(target, v)} />
    </div>
  );
}

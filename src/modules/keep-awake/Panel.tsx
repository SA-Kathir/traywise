import { useState } from "react";
import { commands, events, type KeepAwakeState, type Mode } from "../../bindings";
import { Num, Tile } from "../../shell/Tile";
import { useLive } from "../../shell/useLive";
import { useNow } from "../../shell/useNow";
import type { TileProps } from "..";

// The tile's icon shows its state: a cup while keeping awake, a sleeping moon while not.
const AWAKE = String.fromCodePoint(0xec32);
const ASLEEP = String.fromCodePoint(0xec46);

const MODES: { mode: Mode; label: string }[] = [
  { mode: "off", label: "Off" },
  { mode: "on", label: "On" },
  { mode: "whilePluggedIn", label: "Plugged in" },
];

/** Timer lengths the − and + buttons step through, in minutes. */
const STEPS = [1, 5, 10, 15, 30, 45, 60, 90, 120, 180, 240, 360, 480, 720];

const useKeepAwake = () => useLive(commands.keepAwakeState, events.keepAwakeChanged);

/** Home tile: a click keeps the PC awake until turned off, or lets it sleep again. 2×2, as in the
 *  design: a ring of 24 dots that empties as a timer runs out, and 15 min, 1 hour and ∞ shortcuts. */
export function Tiles({ size, open }: TileProps) {
  const state = useKeepAwake();
  const now = useNow(state?.endsAt != null);
  if (!state) return null;
  const left = state.endsAt === null ? null : Math.max(0, state.endsAt - now);
  const toggle = () => commands.keepAwakeSetMode(state.mode === "off" ? "on" : "off");
  if (size !== "2x2") {
    const detail =
      left !== null
        ? size === "1x1"
          ? clock(left)
          : `${clock(left)} left`
        : state.mode === "whilePluggedIn"
          ? state.active
            ? "While plugged in"
            : "Paused on battery"
          : state.active
            ? "On"
            : size === "1x1"
              ? "Off"
              : "Off · tap to keep awake";
    return (
      <Tile size={size} icon={state.active ? AWAKE : ASLEEP} label="Keep awake" detail={detail} on={state.mode !== "off"} onToggle={toggle} onOpen={open} />
    );
  }
  const total = typeof state.mode === "object" ? state.mode.minutes * 60000 : null;
  // The ring: full while on (a timer's share left while timed), empty while off.
  const lit = state.mode === "off" ? 0 : left !== null && total ? Math.ceil((left / total) * RING) : RING;
  const minutes = left === null ? 0 : Math.ceil(left / 60000);
  const big = left === null ? null : minutes < 60 ? [String(minutes), "min left"] : [clock(left).replace(/:\d\d$/, ""), "h left"];
  return (
    <div className={`tile awake size-2x2${state.mode !== "off" ? " on" : ""}`}>
      <button className="tile-title" title="Details" onClick={open}>
        <span className="tile-icon" aria-hidden>
          {state.active ? AWAKE : ASLEEP}
        </span>
        <span className="tile-label">Keep awake</span>
      </button>
      <button className="awake-main" onClick={toggle} aria-pressed={state.mode !== "off"} title={state.mode === "off" ? "Keep awake" : "Let it sleep"}>
        <Ring lit={lit} />
        <span className="awake-state">
          {big ? (
            <>
              <Num text={big[0]} />
              <span className="tile-detail">{big[1]}</span>
            </>
          ) : (
            <span className="tile-label">{state.mode === "off" ? "Off" : state.active ? "On" : "Paused"}</span>
          )}
        </span>
      </button>
      <span className="tile-detail">
        {state.endsAt !== null
          ? `Until ${new Date(state.endsAt).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}`
          : state.mode === "off"
            ? "Sleeps as usual"
            : state.active
              ? "Until you turn it off"
              : "Paused on battery"}
      </span>
      <span className="awake-quick">
        {QUICK.map(([label, mode]) => (
          <button
            key={label}
            className={JSON.stringify(state.mode) === JSON.stringify(mode) ? "on" : ""}
            onClick={() => commands.keepAwakeSetMode(mode)}
          >
            {label}
          </button>
        ))}
      </span>
    </div>
  );
}

/** The 2×2 tile's shortcuts. */
const QUICK: [string, Mode][] = [
  ["15m", { minutes: 15 }],
  ["1h", { minutes: 60 }],
  ["∞", "on"],
];

const RING = 24;

/** 24 dots in a circle; the first `lit` (clockwise from the top) are lit. */
function Ring({ lit }: { lit: number }) {
  return (
    <svg className="awake-ring" viewBox="0 0 64 64" aria-hidden>
      {Array.from({ length: RING }, (_, i) => {
        const a = (i / RING) * 2 * Math.PI - Math.PI / 2;
        return <circle key={i} cx={32 + 27 * Math.cos(a)} cy={32 + 27 * Math.sin(a)} r={2.6} className={i < lit ? "lit" : undefined} />;
      })}
    </svg>
  );
}

export default function Panel() {
  const state = useKeepAwake();
  if (!state) return null;
  return (
    <>
      <Controls state={state} />
      <label className="row">
        <span className="grow">Keep the display on too</span>
        <input type="checkbox" checked={state.display} onChange={(e) => commands.keepAwakeSetDisplay(e.target.checked)} />
      </label>
      <p className="hint">{status(state)}</p>
      {state.screenOffSleeps && state.active && !state.display && (
        <p className="hint">
          This PC goes to sleep when its screen turns off, whatever apps ask (it uses Windows' Modern Standby). To
          keep it awake, also keep the display on.
        </p>
      )}
    </>
  );
}

/** The modes, and the timer with − and +: on the page and on the 4×2 tile. */
function Controls({ state }: { state: KeepAwakeState }) {
  const [minutes, setMinutes] = useState(30); // the timer length picked, before it's started
  const now = useNow(state.endsAt !== null);

  const left = state.endsAt === null ? null : Math.max(0, state.endsAt - now);
  // While a timer runs, − and + step from the time left; otherwise from the length picked.
  function step(direction: 1 | -1) {
    const from = left === null ? minutes : left / 60000;
    const next =
      direction === 1 ? (STEPS.find((s) => s > from) ?? STEPS[STEPS.length - 1]) : ([...STEPS].reverse().find((s) => s < from) ?? STEPS[0]);
    setMinutes(next);
    if (left !== null) commands.keepAwakeSetMode({ minutes: next });
  }

  return (
    <>
      <div className="segmented">
        {MODES.map(({ mode, label }) => (
          <button key={label} className={state.mode === mode ? "on" : ""} onClick={() => commands.keepAwakeSetMode(mode)}>
            {label}
          </button>
        ))}
      </div>
      <div className="segmented timer">
        <button aria-label="Less time" onClick={() => step(-1)}>
          −
        </button>
        <button
          className={left !== null ? "on" : ""}
          onClick={() => commands.keepAwakeSetMode(left === null ? { minutes } : "off")}
          title={left === null ? "Start the timer" : "Stop the timer"}
        >
          {left === null ? `Timer: ${duration(minutes)}` : <Num text={`${clock(left)} left`} />}
        </button>
        <button aria-label="More time" onClick={() => step(1)}>
          +
        </button>
      </div>
    </>
  );
}

function status(state: KeepAwakeState) {
  if (state.mode === "whilePluggedIn" && !state.active) return "On battery: your PC can sleep until it's plugged in.";
  return state.active ? "Your PC won't go to sleep." : "Your PC sleeps as usual.";
}

/** 90 -> "1 h 30 min" */
function duration(minutes: number) {
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  return [h > 0 && `${h} h`, (m > 0 || h === 0) && `${m} min`].filter(Boolean).join(" ");
}

/** Milliseconds as "1:05:09" or "29:41". */
function clock(ms: number) {
  const total = Math.ceil(ms / 1000);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = String(total % 60).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${s}` : `${m}:${s}`;
}

import { useEffect, useState } from "react";
import { commands, events, type KeepAwakeState, type Mode } from "../../bindings";
import { Tile, TileCard } from "../../shell/Tile";
import { useLive } from "../../shell/useLive";
import keepAwake from ".";
import type { TileProps } from "..";

const MODES: { mode: Mode; label: string }[] = [
  { mode: "off", label: "Off" },
  { mode: "on", label: "On" },
  { mode: "whilePluggedIn", label: "Plugged in" },
];

/** Timer lengths the − and + buttons step through, in minutes. */
const STEPS = [1, 5, 10, 15, 30, 45, 60, 90, 120, 180, 240, 360, 480, 720];

const useKeepAwake = () => useLive(commands.keepAwakeState, events.keepAwakeChanged);

/** Home tile: switches keep-awake on (until turned off) or off; at 4×2, the page's modes and timer. */
export function Tiles({ size, open }: TileProps) {
  const state = useKeepAwake();
  const now = useNow(state?.endsAt != null);
  if (!state) return null;
  const detail =
    state.endsAt !== null
      ? `${clock(Math.max(0, state.endsAt - now))} left`
      : state.mode === "whilePluggedIn"
        ? state.active
          ? "While plugged in"
          : "Paused on battery"
        : state.active
          ? "On"
          : "Off";
  if (size === "4x2")
    return (
      <TileCard size={size} label="Keep awake" detail={detail} onOpen={open}>
        <Controls state={state} />
      </TileCard>
    );
  return (
    <Tile
      size={size}
      icon={keepAwake.icon}
      label="Keep awake"
      detail={detail}
      on={state.mode !== "off"}
      onToggle={() => commands.keepAwakeSetMode(state.mode === "off" ? "on" : "off")}
      onOpen={open}
    />
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
          {left === null ? `Timer: ${duration(minutes)}` : `${clock(left)} left`}
        </button>
        <button aria-label="More time" onClick={() => step(1)}>
          +
        </button>
      </div>
    </>
  );
}

/** The current time, ticking every second while `ticking`. */
function useNow(ticking: boolean) {
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    if (!ticking) return;
    setNow(Date.now());
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, [ticking]);
  return now;
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

import { useEffect, useState } from "react";
import { commands, events, type KeepAwakeState, type Mode } from "../../bindings";

const MODES: { mode: Mode; label: string }[] = [
  { mode: "off", label: "Off" },
  { mode: "on", label: "On" },
  { mode: "whilePluggedIn", label: "Plugged in" },
];

/** Timer lengths the − and + buttons step through, in minutes. */
const STEPS = [1, 5, 10, 15, 30, 45, 60, 90, 120, 180, 240, 360, 480, 720];

export default function Panel() {
  const [state, setState] = useState<KeepAwakeState | null>(null);
  const [minutes, setMinutes] = useState(30); // the timer length picked, before it's started

  useEffect(() => {
    let live = false; // an event beat the first fetch: don't overwrite it with older data
    const unlisten = events.keepAwakeChanged.listen((e) => {
      live = true;
      setState(e.payload);
    });
    commands.keepAwakeState().then((s) => live || setState(s));
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);

  const now = useNow(state?.endsAt != null);
  if (!state) return null;

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
      <label className="row">
        <span className="grow">Keep the display on too</span>
        <input type="checkbox" checked={state.display} onChange={(e) => commands.keepAwakeSetDisplay(e.target.checked)} />
      </label>
      <p className="hint">{status(state)}</p>
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

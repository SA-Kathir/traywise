import { useEffect, useState } from "react";
import { commands, events, type KeepAwakeState, type Mode } from "../../bindings";

const MODES: { mode: Mode; label: string }[][] = [
  [
    { mode: "off", label: "Off" },
    { mode: "on", label: "On" },
    { mode: "whilePluggedIn", label: "Plugged in" },
  ],
  [
    { mode: { minutes: 30 }, label: "30 min" },
    { mode: { minutes: 60 }, label: "1 h" },
    { mode: { minutes: 120 }, label: "2 h" },
  ],
];

const same = (a: Mode, b: Mode) => JSON.stringify(a) === JSON.stringify(b);

export default function Panel() {
  const [state, setState] = useState<KeepAwakeState | null>(null);

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

  if (!state) return null;

  return (
    <>
      {MODES.map((row, i) => (
        <div key={i} className="segmented">
          {row.map(({ mode, label }) => (
            <button
              key={label}
              className={same(state.mode, mode) ? "on" : ""}
              onClick={() => commands.keepAwakeSetMode(mode)}
            >
              {label}
            </button>
          ))}
        </div>
      ))}
      <label className="row">
        <span className="grow">Keep the display on too</span>
        <input
          type="checkbox"
          checked={state.display}
          onChange={(e) => commands.keepAwakeSetDisplay(e.target.checked)}
        />
      </label>
      <Status state={state} />
    </>
  );
}

function Status({ state }: { state: KeepAwakeState }) {
  if (state.endsAt !== null) return <TimeLeft endsAt={state.endsAt} />;
  if (state.mode === "whilePluggedIn" && !state.active)
    return <p className="hint">On battery: your PC can sleep until it's plugged in.</p>;
  return <p className="hint">{state.active ? "Your PC won't go to sleep." : "Your PC sleeps as usual."}</p>;
}

function TimeLeft({ endsAt }: { endsAt: number }) {
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);
  const minutes = Math.max(0, Math.ceil((endsAt - now) / 60000));
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  return <p className="hint">Your PC won't go to sleep for {h > 0 ? `${h} h ${m} min` : `${m} min`}.</p>;
}

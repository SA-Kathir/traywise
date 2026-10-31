import { commands, events, type Battery, type PowerMode } from "../../bindings";
import { Num, Tile, TileCard } from "../../shell/Tile";
import { useLive } from "../../shell/useLive";
import { battery as batteryText } from "../system-monitor/battery";
import type { TileProps } from "..";

// Segoe Fluent Icons glyphs, closest to the design's leaf, gauge and bolt.
const BOLT = String.fromCodePoint(0xe945);
const MODES: { mode: PowerMode; label: string; icon: string }[] = [
  { mode: "efficiency", label: "Efficiency", icon: String.fromCodePoint(0xec0a) },
  { mode: "balanced", label: "Balanced", icon: String.fromCodePoint(0xec49) },
  { mode: "performance", label: "Performance", icon: BOLT },
];

const usePower = () => useLive(commands.powerState, events.powerChanged);

/** "4 h 10 left", "35 min left", "Charging": the 2×1 battery tile's second line. */
function left({ charging, secondsLeft }: Battery) {
  if (charging) return "Charging";
  if (secondsLeft === null) return "On battery";
  const minutes = Math.round(secondsLeft / 60);
  return minutes >= 60 ? `${Math.floor(minutes / 60)} h ${minutes % 60} left` : `${minutes} min left`;
}

/** Under 20 % in the warning colour, under 10 % in the danger colour. */
const low = (percent: number) => (percent < 10 ? " critical" : percent < 20 ? " low" : "");

/** Home tiles: the power mode (a tap cycles it; at 2×1, all three) and the battery. */
export function Tiles({ kind, size, open }: TileProps) {
  const state = usePower();
  if (!state) return null;
  if (kind === "battery") {
    const b = state.battery;
    if (!b) return <Tile size={size} icon={BOLT} label="Battery" detail="No battery" onOpen={open} />;
    if (size === "1x1")
      return (
        <button className={`tile battery size-1x1${low(b.percent)}`} title={`Battery: ${batteryText(b)}`} onClick={open}>
          <Gauge percent={b.percent} />
          <span className="battery-level">
            <Num text={`${b.percent}%`} />
            {b.charging && <span className="icon">{BOLT}</span>}
          </span>
        </button>
      );
    return (
      <TileCard size={size} label="Battery" detail={b.charging ? BOLT : undefined} onOpen={open}>
        <div className={`battery-row${low(b.percent)}`}>
          <span className="battery-big">
            <Num text={`${b.percent}%`} />
          </span>
          <span className="battery-gauge">
            <Gauge percent={b.percent} />
            <span className="tile-detail">{left(b)}</span>
          </span>
        </div>
      </TileCard>
    );
  }
  const mode = state.mode;
  const current = MODES.find((m) => m.mode === mode);
  if (size === "2x1" && current)
    return (
      <TileCard size={size} label="Power mode" detail={current.label} onOpen={open}>
        <Modes mode={current.mode} icons />
      </TileCard>
    );
  const next = current && MODES[(MODES.indexOf(current) + 1) % MODES.length];
  return (
    <Tile
      size={size}
      icon={(current ?? MODES[1]).icon}
      label="Power mode"
      detail={current ? current.label : "Unavailable"}
      onToggle={next && (() => commands.powerSetMode(next.mode))}
      onOpen={open}
    />
  );
}

/** The design's battery cell: two rows of eight dots, lit column by column to the charge, and a nub. */
function Gauge({ percent }: { percent: number }) {
  const lit = Math.max(percent > 0 ? 1 : 0, Math.round((percent / 100) * 8));
  return (
    <svg className="gauge" viewBox="0 0 44 18" aria-hidden>
      <rect x="0.5" y="0.5" width="39" height="17" rx="4" fill="none" stroke="currentColor" />
      <rect x="41" y="6" width="2.5" height="6" rx="1" fill="currentColor" />
      {Array.from({ length: 16 }, (_, i) => (
        <circle key={i} cx={5.5 + (i % 8) * 4.1} cy={i < 8 ? 6.5 : 11.5} r="1.4" className={i % 8 < lit ? "lit" : undefined} />
      ))}
    </svg>
  );
}

function Modes({ mode, icons }: { mode: PowerMode; icons?: boolean }) {
  return (
    <div className={`segmented${icons ? " icons" : ""}`} role="radiogroup" aria-label="Power mode">
      {MODES.map((m) => (
        <button
          key={m.mode}
          role="radio"
          aria-checked={mode === m.mode}
          className={mode === m.mode ? "on" : ""}
          title={m.label}
          onClick={() => commands.powerSetMode(m.mode)}
        >
          <span className="icon" aria-hidden>
            {m.icon}
          </span>
          {!icons && ` ${m.label}`}
        </button>
      ))}
    </div>
  );
}

export default function Panel() {
  const state = usePower();
  if (!state) return null;
  const at = state.energySaverAt;
  return (
    <>
      <h2>Power mode</h2>
      {state.mode ? <Modes mode={state.mode} /> : <p className="hint">Windows offers power modes only on the Balanced power plan.</p>}
      {state.battery && (
        <>
          <h2>Battery</h2>
          <div className={`row battery-row${low(state.battery.percent)}`}>
            <span className="battery-big">
              <Num text={`${state.battery.percent}%`} />
            </span>
            <span className="grow">{left(state.battery)}</span>
          </div>
        </>
      )}
      <div className="row">
        <span className="grow">
          Energy saver
          <br />
          <span className="hint">{state.energySaver ? "On" : at && at < 100 ? `Off · turns on at ${at}%` : "Off"}</span>
        </span>
        <button onClick={() => commands.openSettings("energySaver")}>Open Settings</button>
      </div>
    </>
  );
}

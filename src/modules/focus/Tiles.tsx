import { commands, events, type FocusMode } from "../../bindings";
import { RING, Ring } from "../../shell/Ring";
import { Num, Tile } from "../../shell/Tile";
import { useLive } from "../../shell/useLive";
import { useNow } from "../../shell/useNow";
import type { TileProps } from "..";

// Segoe Fluent Icons glyphs.
const TARGET = String.fromCodePoint(0xe1d2);
const BELL_OFF = String.fromCodePoint(0xe7ed);

/** The 2×2 tile's shortcuts. A click on the others starts the first. */
const QUICK: [string, FocusMode][] = [
  ["25m", { minutes: 25 }],
  ["50m", { minutes: 50 }],
  ["1h", { minutes: 60 }],
];

/** Milliseconds as "24:10" or "1:05:09". */
function clock(ms: number) {
  const total = Math.ceil(ms / 1000);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = String(total % 60).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${s}` : `${m}:${s}`;
}

/** Two tiles, both Windows' Do not disturb: Focus switches it on for a while (25 minutes from a
 *  click), with a countdown; Do not disturb is a plain on/off switch. Both show changes made in
 *  Windows too. */
export default function Tiles({ kind, size }: TileProps) {
  const state = useLive(commands.focusState, events.focusChanged);
  const now = useNow(state?.endsAt != null);
  if (!state) return null;
  const left = state.endsAt === null ? null : Math.max(0, state.endsAt - now);
  const set = (mode: FocusMode) => commands.focusSet(mode);

  if (!state.available)
    return <Tile size={size} icon={kind === "dnd" ? BELL_OFF : TARGET} label={kind === "dnd" ? "Do not disturb" : "Focus"} detail={size === "1x1" ? "N/A" : "Not available"} />;

  if (kind === "dnd")
    return (
      <Tile
        size={size}
        icon={BELL_OFF}
        label="Do not disturb"
        detail={state.on ? "On" : "Off"}
        on={state.on}
        onToggle={() => set(state.on ? "off" : "on")}
      />
    );

  const toggle = () => set(state.on ? "off" : QUICK[0][1]);
  if (size !== "2x2") {
    // 1×1 has room for minutes only: "24m".
    const detail = left !== null ? (size === "1x1" ? `${Math.ceil(left / 60000)}m` : `${clock(left)} left`) : state.on ? "On" : "Off";
    return <Tile size={size} icon={TARGET} label="Focus" detail={detail} on={state.on} onToggle={toggle} />;
  }

  // The ring: a timer's share left while timed, full while on without one, empty while off.
  const lit = !state.on ? 0 : left !== null && state.minutes ? Math.ceil((left / (state.minutes * 60000)) * RING) : RING;
  const minutes = left === null ? 0 : Math.ceil(left / 60000);
  return (
    <div className={`tile awake size-2x2${state.on ? " on" : ""}`}>
      <span className="tile-title">
        <span className="tile-icon" aria-hidden>
          {TARGET}
        </span>
        <span className="tile-label">Focus</span>
      </span>
      <button className="awake-main" onClick={toggle} aria-pressed={state.on} title={state.on ? "Turn off" : "Focus for 25 minutes"}>
        <Ring lit={lit} />
        <span className="awake-state">
          {left !== null ? (
            <>
              <Num text={String(minutes)} />
              <span className="tile-detail">min left</span>
            </>
          ) : (
            <span className="tile-label">{state.on ? "On" : "Off"}</span>
          )}
        </span>
      </button>
      <span className="tile-detail">
        <span className="focus-bell" aria-hidden>
          {BELL_OFF}
        </span>{" "}
        {state.on ? "Alerts muted" : "Mutes alerts"}
      </span>
      <span className="awake-quick">
        {QUICK.map(([label, mode]) => (
          <button key={label} className={typeof mode === "object" && mode.minutes === state.minutes && left !== null ? "on" : ""} onClick={() => set(mode)}>
            {label}
          </button>
        ))}
      </span>
    </div>
  );
}

import { commands, events, type FocusAction, type FocusMode, type FocusPreset } from "../../bindings";
import { Num, Tile } from "../../shell/Tile";
import { useLive } from "../../shell/useLive";
import { useNow } from "../../shell/useNow";
import { useSettings } from "../../shell/SettingsContext";
import type { TileProps } from "..";
import { Arc, Hourglass } from "./Gauges";

// Segoe Fluent Icons glyphs.
const TARGET = String.fromCodePoint(0xe1d2);
const BELL_OFF = String.fromCodePoint(0xe7ed);
const PAUSE = String.fromCodePoint(0xe769);
const PLAY = String.fromCodePoint(0xe768);
const NEXT = String.fromCodePoint(0xe893);
const STOP = String.fromCodePoint(0xe71a);

const FALLBACK: FocusPreset = { focus: 25, rest: 5 };
const pad = (n: number) => String(n).padStart(2, "0");

/** Seconds as "24:10" (minutes can pass 59: "90:00"). */
function clock(seconds: number) {
  const s = Math.ceil(seconds);
  return `${pad(Math.floor(s / 60))}:${pad(s % 60)}`;
}

/** Two tiles: Focus is a Pomodoro-style cycle (focus and rest rounds that can switch Do not disturb
 *  on while they run); Do not disturb is a plain on/off switch that never affects the cycle. Both
 *  show changes made in Windows too. */
export default function Tiles({ kind, size, open }: TileProps) {
  const state = useLive(commands.focusState, events.focusChanged);
  const { settings } = useSettings();
  const timed = state?.phase != null;
  const running = timed && state.leftMs == null;
  const now = useNow(running);
  if (!state) return null;

  if (!state.available)
    return <Tile size={size} icon={kind === "dnd" ? BELL_OFF : TARGET} label={kind === "dnd" ? "Do not disturb" : "Focus"} detail={size === "1x1" ? "N/A" : "Not available"} />;

  if (kind === "dnd") {
    const set = (mode: FocusMode) => commands.focusSet(mode);
    return <Tile size={size} icon={BELL_OFF} label="Do not disturb" detail={state.on ? "On" : "Off"} on={state.on} onToggle={() => set(state.on ? "off" : "on")} />;
  }

  const start = (p: FocusPreset) => commands.focusStart(p.focus, p.rest);
  const control = (action: FocusAction) => commands.focusControl(action);
  const preset = settings.focusPresets[0] ?? FALLBACK;
  // A tap on the tile starts the first preset, or pauses or resumes.
  const toggle = () => (timed ? control(running ? "pause" : "resume") : start(preset));

  const total = (state.minutes ?? 0) * 60;
  const left = !timed ? 0 : state.leftMs != null ? state.leftMs / 1000 : Math.max(0, ((state.endsAt ?? now) - now) / 1000);
  const rest = state.phase === "rest" || state.phase === "longRest";
  const word = state.phase === "longRest" ? "Long rest" : rest ? "Rest" : "In focus";

  if (size !== "2x2" && size !== "4x2") {
    // 1×1 has room for minutes only: "24m".
    const detail = !timed ? "Off" : !running ? "Paused" : size === "1x1" ? `${Math.ceil(left / 60)}m` : `${clock(left)} ${rest ? "rest" : "left"}`;
    return <Tile size={size} icon={TARGET} label="Focus" detail={detail} on={timed} onToggle={toggle} />;
  }

  // Arc: focus empties it from the right, a rest refills it from the left.
  const lit = !timed ? 0 : Math.ceil((rest ? 1 - left / total : left / total) * 19);
  const time = clock(left);
  const glass = settings.focusStyle === "hourglass";
  const [mm, ss] = time.split(":");
  const round = state.round ?? 0;
  return (
    <div className={`tile pomo size-${size} ${glass ? "glass" : "arc"}${timed ? " on" : ""}${rest ? " rest" : ""}`}>
      <span className="tile-title">
        <span className="tile-icon" aria-hidden>
          {TARGET}
        </span>
        <span className="tile-label">Focus</span>
        {timed && (
          <span className="pomo-rounds" title={`Round ${round} of 4`}>
            {[1, 2, 3, 4].map((r) => (
              <i key={r} className={r < round ? "done" : r === round ? "cur" : undefined} />
            ))}
          </span>
        )}
      </span>
      <button className="pomo-main" onClick={toggle} title={timed ? (running ? "Pause" : "Resume") : `Start ${preset.focus} focus, ${preset.rest} rest`}>
        {glass ? <Hourglass elapsed={total - left} total={total || 1} off={!timed} /> : <Arc lit={lit} />}
        {timed ? (
          <span className="pomo-read">
            {glass && size === "2x2" ? (
              <>
                <Num text={mm} />
                <Num text={ss} />
              </>
            ) : (
              <Num text={time} />
            )}
            {(size === "4x2" || !glass) && (
              <span className="tile-detail">{size === "4x2" ? `${word} · round ${round} of 4` : running ? word : "Paused"}</span>
            )}
          </span>
        ) : (
          <span className="pomo-read idle">
            <span className="tile-label">Ready</span>
            <span className="tile-detail">
              {preset.focus} min · {preset.rest} rest
            </span>
          </span>
        )}
      </button>
      <span className="pomo-keys">
        {timed ? (
          <>
            <button className="glyph" aria-label={running ? "Pause" : "Resume"} title={running ? "Pause" : "Resume"} onClick={toggle}>
              {running ? PAUSE : PLAY}
            </button>
            <button className="glyph" aria-label="Next phase" title="Skip to the next phase" onClick={() => control("skip")}>
              {NEXT}
            </button>
            <button className="glyph" aria-label="End" title="End" onClick={() => control("end")}>
              {STOP}
            </button>
          </>
        ) : (
          <>
            {settings.focusPresets.slice(0, 2).map((p) => (
              <button key={`${p.focus}/${p.rest}`} title={`${p.focus} focus, ${p.rest} rest`} onClick={() => start(p)}>
                {p.focus}/{p.rest}
              </button>
            ))}
            <button aria-label="More lengths" title="More lengths" onClick={open}>
              +
            </button>
          </>
        )}
      </span>
    </div>
  );
}

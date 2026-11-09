import { useEffect, useRef, useState, type PointerEvent, type ReactNode } from "react";
import { useSend } from "./Slider";
import { Num } from "./Tile";

// The volume and brightness dials: a 270° arc open at the bottom. 1×1 is a ring (hover: it slides
// up and shows − 55 % +); 2×2 is a big arc you drag. Scrolling over either changes the level. The
// Dot look draws the arc as dots, the others as a solid arc; the drag handle only shows on hover.

const START = 135;
const SWEEP = 270;
const R = 38;
const clamp = (v: number) => Math.max(0, Math.min(100, Math.round(v)));
const at = (deg: number): [number, number] => [50 + R * Math.cos((deg * Math.PI) / 180), 50 + R * Math.sin((deg * Math.PI) / 180)];

function Arc({ level, dots }: { level: number; dots: number }) {
  const [x1, y1] = at(START);
  const [x2, y2] = at(START + SWEEP);
  const d = `M${x1} ${y1}A${R} ${R} 0 1 1 ${x2} ${y2}`;
  const [tx, ty] = at(START + (SWEEP * level) / 100);
  const lead = Math.round((level / 100) * (dots - 1));
  return (
    <svg className="dial-arc" viewBox="0 0 100 100" aria-hidden>
      <g className="dial-solid">
        <path className="dial-track" d={d} pathLength={100} />
        {level > 0 && <path className="dial-fill" d={d} pathLength={100} strokeDasharray={`${level} 100`} />}
        {level > 0 && <circle className="dial-thumb" cx={tx} cy={ty} r={7} />}
      </g>
      <g className="dial-dots">
        {Array.from({ length: dots }, (_, i) => {
          const [x, y] = at(START + (SWEEP * i) / (dots - 1));
          const lit = level > 0 && i <= lead;
          return <circle key={i} cx={x} cy={y} className={lit ? (i === lead ? "lit lead" : "lit") : undefined} />;
        })}
      </g>
    </svg>
  );
}

/** A volume or brightness tile at 1×1 or 2×2. `onToggle` (mute) is optional: without it, clicking
 *  the ring or the number opens the module's page, as the other tiles do. */
export function DialTile({
  size,
  noun,
  icon,
  level,
  off,
  detail,
  onChange,
  onToggle,
  open,
}: {
  size: "1x1" | "2x2";
  noun: string;
  /** A Segoe Fluent Icons glyph (the crossed speaker when muted). */
  icon: ReactNode;
  level: number;
  off?: boolean;
  /** The device or screen, under the number at 2×2. */
  detail?: string;
  onChange: (v: number) => void;
  onToggle?: () => void;
  open?: () => void;
}) {
  const big = size === "2x2";
  // While dragging, the arc follows the pointer, not the slightly older value Windows echoes back.
  const [dragged, setDragged] = useState<number | null>(null);
  const send = useSend(onChange);
  const shown = off ? 0 : (dragged ?? level);
  const root = useRef<HTMLDivElement>(null);
  const now = useRef(level);
  now.current = level;
  const tick = useRef(0);

  // Scrolling over the tile: one step per 50 px, and the panel doesn't scroll under it.
  useEffect(() => {
    const el = root.current!;
    const wheel = (e: WheelEvent) => {
      e.preventDefault();
      tick.current -= e.deltaY / 50;
      const step = Math.trunc(tick.current);
      if (step === 0) return;
      tick.current -= step;
      now.current = clamp(now.current + step);
      send(now.current);
    };
    el.addEventListener("wheel", wheel, { passive: false });
    return () => el.removeEventListener("wheel", wheel);
  }, [send]);

  // The pointer's angle around the arc's centre; the gap at the bottom snaps to the nearer end.
  const pick = (e: PointerEvent<HTMLDivElement>) => {
    const r = e.currentTarget.getBoundingClientRect();
    const deg = (Math.atan2(e.clientY - r.top - r.height / 2, e.clientX - r.left - r.width / 2) * 180) / Math.PI;
    const rel = (deg - START + 720) % 360;
    const v = rel <= SWEEP ? (rel / SWEEP) * 100 : rel - SWEEP < (360 - SWEEP) / 2 ? 100 : 0;
    setDragged(clamp(v));
    send(clamp(v));
  };
  const step = (by: number) => send(clamp(level + by));
  const text = off ? "Muted" : `${shown}%`;
  const label = off ? `${noun} muted (click to unmute)` : `${noun} ${shown}`;

  if (!big)
    return (
      <div ref={root} className={`tile dial size-1x1${off ? " off" : ""}`}>
        <button className="dial-ring" title={label} onClick={onToggle ?? open}>
          <Arc level={shown} dots={17} />
          <span className="dial-icon" aria-hidden>
            {icon}
          </span>
        </button>
        <div className="dial-row">
          <button className="dial-step" aria-label={`Lower ${noun}`} onClick={() => step(-5)}>
            −
          </button>
          <span className="dial-value">{off ? text : <Num text={text} />}</span>
          <button className="dial-step" aria-label={`Raise ${noun}`} onClick={() => step(5)}>
            +
          </button>
        </div>
      </div>
    );
  return (
    <div ref={root} className={`tile dial size-2x2${off ? " off" : ""}`}>
      <button className="tile-title" title="Details" onClick={open}>
        <span className="dial-icon icon" aria-hidden>
          {icon}
        </span>
        <span className="tile-label">{noun}</span>
      </button>
      <div
        className="dial-hit"
        role="slider"
        aria-label={noun}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={shown}
        onPointerDown={(e) => {
          e.currentTarget.setPointerCapture(e.pointerId);
          pick(e);
        }}
        onPointerMove={(e) => e.currentTarget.hasPointerCapture(e.pointerId) && pick(e)}
        onPointerUp={() => setDragged(null)}
        onPointerCancel={() => setDragged(null)}
      >
        <Arc level={shown} dots={33} />
        <button className="dial-center" title={onToggle ? label : "Details"} onClick={onToggle ?? open} onPointerDown={(e) => e.stopPropagation()}>
          <span className="dial-big">{off ? <span className="dial-muted">Muted</span> : <Num text={String(shown)} />}</span>
        </button>
      </div>
      {detail && <span className="tile-detail">{detail}</span>}
    </div>
  );
}

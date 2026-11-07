import { DOTS, sandAt } from "./hourglass";

// The two ways Focus draws its countdown, in dots. Which one is a setting (Focus page › Tile style).

const ARC = 19;

/** A half ring of 19 dots, left to right over the top; the first `lit` are lit. */
export function Arc({ lit }: { lit: number }) {
  return (
    <svg className="pomo-arc" viewBox="0 0 116 60" aria-hidden>
      {Array.from({ length: ARC }, (_, i) => {
        const a = Math.PI + (i / (ARC - 1)) * Math.PI;
        return <circle key={i} cx={58 + 52 * Math.cos(a)} cy={57 + 52 * Math.sin(a)} r={2.5} className={i < lit ? "lit" : undefined} />;
      })}
    </svg>
  );
}

const PITCH = 6;
const DOT = 4.4;

/** The hourglass, `elapsed` seconds into a phase of `total` seconds (empty when `off`). */
export function Hourglass({ elapsed, total, off, turned }: { elapsed: number; total: number; off?: boolean; turned?: boolean }) {
  const sand = off ? [] : sandAt(elapsed, total);
  return (
    <svg className={turned ? "pomo-glass turn" : "pomo-glass"} viewBox={`0 0 ${10 * PITCH + DOT} ${12 * PITCH + DOT}`} aria-hidden>
      {DOTS.map((d, i) => (
        <circle key={i} cx={d.c * PITCH + DOT / 2} cy={d.r * PITCH + DOT / 2} r={DOT / 2} className={sand[i] || undefined} />
      ))}
    </svg>
  );
}

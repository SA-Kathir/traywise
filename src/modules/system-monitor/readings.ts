import type { Reading } from "../../bindings.ts";

/** The readings a wide tile can show, in the order tapping its pills cycles through them. */
export const READING_CHOICES = [
  ["cpu", "CPU"],
  ["memory", "RAM"],
  ["download", "Download"],
  ["upload", "Upload"],
] as const;

type Pair = [Reading, Reading];
const known = (r: unknown) => READING_CHOICES.some(([id]) => id === r);

/** A tile's own pair if it is a proper one (exactly two known readings), else the default: a
 *  hand-edited settings file can't put anything else on screen. */
export const pairOrDefault = (own: Pair | null | undefined, fallback: Pair): Pair =>
  Array.isArray(own) && own.length === 2 && own.every(known) ? own : fallback;

/** The pair with one side moved on to its next reading; the other side stays. Always two readings. */
export function cycle(now: Pair, side: 0 | 1): Pair {
  const ids = READING_CHOICES.map(([id]) => id);
  const next: Pair = [now[0], now[1]];
  next[side] = ids[(ids.indexOf(now[side]) + 1) % ids.length];
  return next;
}

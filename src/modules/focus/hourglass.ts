// The Focus hourglass: 11 × 13 dots, sand in the top bulb, a pile in the bottom bulb, and grains in
// between. Pure, so it can be tested; the tile only draws what this says.
//
// There is one grain per bottom dot (46). Grain i leaves the neck some rows before its dot is due and
// falls one row a second, so it lands exactly when the time says that dot is full; once landed it
// stays for good. Nothing ever moves up.

export type Sand = "" | "sand" | "dim" | "pile" | "grain";
export type Dot = { c: number; r: number };

const MID = 6; // the neck's row
const top: Dot[] = [];
const bottom: Dot[] = [];
for (let r = 0; r < 13; r++) {
  const width = Math.min(11, 1 + 2 * Math.abs(r - MID));
  const first = (11 - width) / 2;
  for (let k = 0; k < width; k++) (r < MID ? top : r > MID ? bottom : []).push({ c: first + k, r });
}
// Next to the neck first, the middle of a row last: the sand's surface dips in the middle.
top.sort((a, b) => b.r - a.r || Math.abs(b.c - 5) - Math.abs(a.c - 5));
// Bottom row first, the middle of a row first: the pile peaks.
bottom.sort((a, b) => b.r - a.r || Math.abs(a.c - 5) - Math.abs(b.c - 5));

/** Every dot in drawing order: the top bulb, the neck, the bottom bulb. */
export const DOTS: Dot[] = [...top, { c: 5, r: MID }, ...bottom];

/** What each of `DOTS` shows, `elapsed` seconds into a phase of `total` seconds. */
export function sandAt(elapsed: number, total: number): Sand[] {
  const t = Math.min(Math.max(elapsed, 0), total);
  const grains = bottom.length;
  let landed = 0;
  let left = 0; // grains that have left the top
  const falling = new Set<number>();
  bottom.forEach((dot, i) => {
    const due = ((i + 1) * total) / grains;
    const rows = dot.r - MID;
    const fall = Math.min(rows, due); // a row a second, squeezed if the phase is very short
    const start = due - fall;
    if (t >= due) landed++;
    if (t >= start) left++;
    if (t >= start && t < due && rows > 1) {
      const row = Math.min(rows - 1, Math.floor(((t - start) / fall) * rows) + 1);
      // A straight line from the neck to its dot, which always stays inside the bulb.
      falling.add((MID + row) * 20 + Math.round(5 + ((dot.c - 5) * row) / rows));
    }
  });
  const sand = top.length - left;
  const out: Sand[] = top.map((_, i) => (i < sand ? "sand" : ""));
  // A dimmer dot sinks through the sand above the neck, then starts again: a sign of life that never goes below it.
  const phase = Math.floor(t) % 4;
  if (phase < 3) {
    const i = top.findIndex((d) => d.c === 5 && d.r === 3 + phase);
    if (i >= 0 && i < sand) out[i] = "dim";
  }
  out.push(t < total ? "sand" : "");
  bottom.forEach((dot, i) => out.push(i < landed ? "pile" : falling.has(dot.r * 20 + dot.c) ? "grain" : ""));
  return out;
}

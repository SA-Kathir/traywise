import type { BoardTile } from "../bindings.ts";

// The home board: tiles on a grid four cells wide that grows downward. Pure functions, tested in
// board.test.ts. Every function returns the tiles in reading order (top to bottom, left to right),
// which is also their order on screen, for Tab and screen readers.
//
// Two ways to settle tiles after a change:
// - auto-arrange: tiles keep their order, and each takes the first free spot, so a later small tile
//   fills a hole and the board never has gaps;
// - free placement: tiles rise as far as they can in their column, so there's never a gap above
//   one. A tile in the way of a moved one goes to whichever is closer: the nearest free spot (like
//   Android's home screen; often the spot the moved one left, so they swap), or just below the moved
//   one (like web dashboard grids), from where it rises again once there's room.

export const COLUMNS = 4;

/** A tile on the board: `w` × `h` cells, with its top-left cell at column `x`, row `y` (saved as is in settings). */
export type Placed = BoardTile;

const byPosition = (a: Placed, b: Placed) => a.y - b.y || a.x - b.x;

const overlaps = (a: Placed, b: Placed) =>
  a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;

const fits = (t: Placed, others: Placed[]) =>
  t.x >= 0 && t.y >= 0 && t.x + t.w <= COLUMNS && others.every((o) => !overlaps(t, o));

/** The first spot in reading order where `t` fits. */
function firstFree(t: Placed, others: Placed[]): Placed {
  if (t.w > COLUMNS) throw new Error(`tile ${t.id} is wider than the board`);
  for (let y = 0; ; y++)
    for (let x = 0; x + t.w <= COLUMNS; x++) if (fits({ ...t, x, y }, others)) return { ...t, x, y };
}

/** The free spot closest to where `t` is now (the earlier one in reading order on a tie), and how far. */
function nearestFree(t: Placed, others: Placed[]): [Placed, number] {
  // Below every tile there's always room, so the search ends there.
  const bottom = Math.max(0, ...others.map((o) => o.y + o.h));
  let best = { ...t, x: 0, y: bottom };
  let bestDistance = Infinity;
  for (let y = 0; y <= bottom; y++)
    for (let x = 0; x + t.w <= COLUMNS; x++) {
      const distance = Math.hypot(x - t.x, y - t.y);
      if (distance < bestDistance && fits({ ...t, x, y }, others)) [best, bestDistance] = [{ ...t, x, y }, distance];
    }
  return [best, bestDistance];
}

/** Auto-arrange: each tile, in order, takes the first free spot. */
export function pack(tiles: Placed[]): Placed[] {
  const out: Placed[] = [];
  for (const t of tiles) out.push(firstFree(t, out));
  return out.sort(byPosition);
}

/** Free placement: `fixed` stays exactly where it is; every other tile, top to bottom, is pushed
 *  down past anything in its way, then rises as far as it can in its column. */
function settle(fixed: Placed[], rest: Placed[]): Placed[] {
  const out = [...fixed];
  for (const o of [...rest].sort(byPosition)) {
    let t = clampX(o);
    while (!fits(t, out)) t = { ...t, y: t.y + 1 };
    while (t.y > 0 && fits({ ...t, y: t.y - 1 }, out)) t = { ...t, y: t.y - 1 };
    out.push(t);
  }
  return out.sort(byPosition);
}

/** Free placement with nothing held in place: every tile rises as far as it can. */
export const compact = (tiles: Placed[]) => settle([], tiles);

const clampX = (t: Placed) => ({ ...t, x: Math.max(0, Math.min(t.x, COLUMNS - t.w)), y: Math.max(0, t.y) });

/** Puts tile `id` with its top-left cell at (`x`, `y`). With auto-arrange, it takes that place in
 *  the order (before the tile that's there now) and the board packs again. With free placement it
 *  stays exactly there (it's under the pointer while dragging); a tile in the way swaps into the spot
 *  it left, if it fits there, and the rest settle around it. */
export function move(tiles: Placed[], id: string, x: number, y: number, auto: boolean): Placed[] {
  const t = tiles.find((t) => t.id === id);
  if (!t) return tiles;
  const moved = clampX({ ...t, x, y });
  const rest = tiles.filter((o) => o !== t);
  if (!auto) {
    const others = rest.map((o) => {
      if (!overlaps(o, moved)) return o;
      const [free, distance] = nearestFree(o, [moved, ...rest.filter((r) => r !== o)]);
      // Pushed below the moved tile: settle does that from where it is now.
      return distance <= moved.y + moved.h - o.y ? free : o;
    });
    return settle([moved], others);
  }
  const i = rest.findIndex((o) => byPosition(o, moved) >= 0);
  rest.splice(i < 0 ? rest.length : i, 0, moved);
  return pack(rest);
}

/** Moves tile `id` earlier (-1) or later (+1) in the order, past as many tiles as it takes for it to
 *  land somewhere else (a small tile may fill the same hole after one swap). For the keyboard with
 *  auto-arrange, where "one cell left" may not change the order. */
export function shift(tiles: Placed[], id: string, by: -1 | 1): Placed[] {
  const start = tiles.find((t) => t.id === id);
  const order = [...tiles];
  for (let i = order.findIndex((t) => t.id === id); start && i + by >= 0 && i + by < order.length; i += by) {
    [order[i], order[i + by]] = [order[i + by], order[i]];
    const packed = pack(order);
    const now = packed.find((t) => t.id === id)!;
    if (now.x !== start.x || now.y !== start.y) return packed;
  }
  return tiles;
}

export function resize(tiles: Placed[], id: string, w: number, h: number, auto: boolean): Placed[] {
  const t = tiles.find((t) => t.id === id);
  if (!t) return tiles;
  const resized = clampX({ ...t, w, h });
  if (auto) return pack(tiles.map((o) => (o === t ? resized : o)));
  return settle([resized], tiles.filter((o) => o !== t));
}

/** Adds a tile at the end (auto-arrange) or in the first free spot (free placement). */
export function add(tiles: Placed[], id: string, w: number, h: number, auto: boolean): Placed[] {
  const t = { id, w, h, x: 0, y: 0 };
  return auto ? pack([...tiles, t]) : [...tiles, firstFree(t, tiles)].sort(byPosition);
}

export function remove(tiles: Placed[], id: string, auto: boolean): Placed[] {
  const rest = tiles.filter((t) => t.id !== id);
  return auto ? pack(rest) : compact(rest);
}

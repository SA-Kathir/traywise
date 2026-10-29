// The home board: tiles on a grid four cells wide that grows downward. Pure functions, tested in
// board.test.ts. Every function returns the tiles in reading order (top to bottom, left to right),
// which is also their order on screen, for Tab and screen readers.
//
// Two ways to settle tiles after a change:
// - auto-arrange: tiles keep their order, and each takes the first free spot, so a later small tile
//   fills a hole and the board never has gaps;
// - free placement: tiles stay where they're put, and one that's in the way moves to the nearest
//   free spot.

export const COLUMNS = 4;

/** A tile on the board: `w` × `h` cells, with its top-left cell at column `x`, row `y`. */
export type Placed = { id: string; w: number; h: number; x: number; y: number };

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

/** The free spot closest to where `t` is now (the earlier one in reading order on a tie). */
function nearestFree(t: Placed, others: Placed[]): Placed {
  // Below every tile there's always room, so the search ends there.
  const bottom = Math.max(0, ...others.map((o) => o.y + o.h));
  let best: Placed | undefined;
  let bestDistance = Infinity;
  for (let y = 0; y <= bottom; y++)
    for (let x = 0; x + t.w <= COLUMNS; x++) {
      const distance = (x - t.x) ** 2 + (y - t.y) ** 2;
      if (distance < bestDistance && fits({ ...t, x, y }, others)) [best, bestDistance] = [{ ...t, x, y }, distance];
    }
  return best!;
}

/** Auto-arrange: each tile, in order, takes the first free spot. */
export function pack(tiles: Placed[]): Placed[] {
  const out: Placed[] = [];
  for (const t of tiles) out.push(firstFree(t, out));
  return out.sort(byPosition);
}

/** Free placement: `fixed` stays put; any other tile in the way moves to the nearest free spot. */
function bump(fixed: Placed[], rest: Placed[]): Placed[] {
  // Everything not in the way stays first, so a moved tile can't land on one that hasn't been placed yet.
  const out = [...fixed];
  const inTheWay: Placed[] = [];
  for (const o of rest) (fits(o, fixed) ? out : inTheWay).push(o);
  for (const o of inTheWay.sort(byPosition)) out.push(nearestFree(o, out));
  return out.sort(byPosition);
}

const clampX = (t: Placed) => ({ ...t, x: Math.max(0, Math.min(t.x, COLUMNS - t.w)), y: Math.max(0, t.y) });

/** Drops tile `id` with its top-left cell at (`x`, `y`). With auto-arrange, it takes that place in
 *  the order (before the tile that's there now) and the board packs again. */
export function move(tiles: Placed[], id: string, x: number, y: number, auto: boolean): Placed[] {
  const t = tiles.find((t) => t.id === id);
  if (!t) return tiles;
  const moved = clampX({ ...t, x, y });
  const rest = tiles.filter((o) => o !== t);
  if (!auto) return bump([moved], rest);
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
  return bump([resized], tiles.filter((o) => o !== t));
}

/** Adds a tile at the end (auto-arrange) or in the first free spot (free placement). */
export function add(tiles: Placed[], id: string, w: number, h: number, auto: boolean): Placed[] {
  const t = { id, w, h, x: 0, y: 0 };
  return auto ? pack([...tiles, t]) : [...tiles, firstFree(t, tiles)].sort(byPosition);
}

export function remove(tiles: Placed[], id: string, auto: boolean): Placed[] {
  const rest = tiles.filter((t) => t.id !== id);
  return auto ? pack(rest) : rest;
}

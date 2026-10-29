// Run with `pnpm test` (Node runs TypeScript directly). Boards are drawn as text, rows separated by
// spaces: each cell is its tile's id, "." is empty.
import { add, move, pack, remove, resize, shift, type Placed } from "./board.ts";

function draw(tiles: Placed[]): string {
  const rows: string[][] = [];
  for (const t of tiles)
    for (let y = t.y; y < t.y + t.h; y++)
      for (let x = t.x; x < t.x + t.w; x++) {
        rows[y] ??= [".", ".", ".", "."];
        if (rows[y][x] !== ".") throw new Error(`${t.id} overlaps ${rows[y][x]}`);
        rows[y][x] = t.id;
      }
  return Array.from(rows, (r) => (r ? r.join("") : "....")).join(" ");
}

function check(name: string, tiles: Placed[], expected: string) {
  const got = draw(tiles);
  if (got !== expected) throw new Error(`${name}: expected "${expected}", got "${got}"`);
  const sorted = [...tiles].sort((a, b) => a.y - b.y || a.x - b.x);
  if (sorted.some((t, i) => t !== tiles[i])) throw new Error(`${name}: not in reading order`);
}

const tile = (id: string, w: number, h: number): Placed => ({ id, w, h, x: 0, y: 0 });

// Auto-arrange: c (1×1) fills the hole beside a, ahead of the wide b.
const board = pack([tile("a", 1, 1), tile("b", 4, 2), tile("c", 1, 1), tile("d", 2, 2)]);
check("pack", board, "ac.. bbbb bbbb dd.. dd..");
check("pack is stable", pack(board), "ac.. bbbb bbbb dd.. dd..");

// Auto-arrange: dropping d on a's cell puts d first in the order.
check("move, auto", move(board, "d", 0, 0, true), "ddac dd.. bbbb bbbb");
// Free placement: c takes a's cell, and a steps aside to the nearest free one.
check("move, free", move(board, "c", 0, 0, false), "ca.. bbbb bbbb dd.. dd..");
// Free placement keeps gaps: b, in d's way, moves down and leaves a hole.
const gappy = move(board, "d", 2, 0, false);
check("move, free, gap", gappy, "acdd ..dd bbbb bbbb");

// Keyboard: d one step earlier lands beside c.
check("shift", shift(board, "d", -1), "acdd ..dd bbbb bbbb");
// c always fills the hole beside a, so there's no later place it can show up in.
check("shift, no visible move", shift(board, "c", 1), "ac.. bbbb bbbb dd.. dd..");
check("shift, at the start", shift(board, "a", -1), "ac.. bbbb bbbb dd.. dd..");

check("resize, auto", resize(board, "a", 2, 1, true), "aac. bbbb bbbb dd.. dd..");
// Free placement: a grows over c's cell; c goes to the nearest free cell, beside d.
check("resize, free", resize(board, "a", 4, 1, false), "aaaa bbbb bbbb ddc. dd..");
// A tile at the right edge that grows wider moves left to stay on the board.
const right = move(board, "c", 3, 0, false);
check("move to the edge", right, "a..c bbbb bbbb dd.. dd..");
check("resize at the edge", resize(right, "c", 2, 1, false), "a.cc bbbb bbbb dd.. dd..");

check("add, auto", add(board, "e", 1, 1, true), "ace. bbbb bbbb dd.. dd..");
check("add, free, into a gap", add(gappy, "e", 2, 1, false), "acdd eedd bbbb bbbb");

check("remove, auto", remove(board, "a", true), "c... bbbb bbbb dd.. dd..");
check("remove, free", remove(board, "a", false), ".c.. bbbb bbbb dd.. dd..");

console.log("board: all checks passed");

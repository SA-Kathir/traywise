import { Suspense, useLayoutEffect, useRef, useState, type KeyboardEvent, type PointerEvent } from "react";
import type { ModuleId } from "../bindings";
import { defaultBoard, modules, tileModule, type Size } from "../modules";
import { add, COLUMNS, compact, move, pack, remove, resize, shift, type Placed } from "./board";

// Segoe Fluent Icons glyphs.
const MINUS = String.fromCodePoint(0xe738);
const RESIZE = String.fromCodePoint(0xe740); // "Full screen": two corners

/** Holding a tile this long (without moving) starts edit mode. */
const LONG_PRESS_MS = 500;
/** A pointer that moves this far (px) is a drag, not a click or a long-press. */
const DRAG_PX = 5;
/** How long a tile takes to glide to its new place, to appear, and to go. */
const MOVE_MS = 180;
const APPEAR_MS = 260;
const REMOVE_MS = 160;
const EASE = "cubic-bezier(0.2, 0.8, 0.2, 1)";
const reducedMotion = () => matchMedia("(prefers-reduced-motion: reduce)").matches;

const cells = (size: Size) => size.split("x").map(Number) as [w: number, h: number];
const sizeOf = (t: Placed) => `${t.w}x${t.h}` as Size;
const kindOf = (id: string) => id.slice(id.indexOf("/") + 1);
const info = (id: string) => modules[tileModule(id)].tiles[kindOf(id)];

/** A new user's board: the default tiles of the modules that are on. */
export function startingBoard(enabled: ModuleId[]): Placed[] {
  return pack(
    defaultBoard.filter(([id]) => enabled.includes(tileModule(id))).map(([id, size]) => placed(id, size)),
  );
}

/** A tile not yet on the board, at its size on the default board (or its smallest). */
export function newTile(id: string): Placed {
  return placed(id, defaultBoard.find(([d]) => d === id)?.[1] ?? info(id).sizes[0]);
}

/** The board to show: the saved one (or the default, before it's first changed), less any tile
 *  this version doesn't know (a hand-edited or newer settings file) or whose module is off. A size
 *  the tile no longer comes in (Keep awake's old 2×2) becomes its default size. */
export function boardToShow(saved: Placed[] | null, enabled: ModuleId[], auto: boolean): Placed[] {
  if (!saved) return startingBoard(enabled);
  const seen = new Set<string>();
  const known = saved.flatMap((t) => {
    const module = tileModule(t.id);
    const kind = enabled.includes(module) && !seen.has(t.id) && modules[module].tiles[kindOf(t.id)];
    seen.add(t.id);
    if (!kind) return [];
    return [kind.sizes.includes(sizeOf(t)) ? t : { ...newTile(t.id), x: t.x, y: t.y }];
  });
  // A resized tile may now overlap its neighbours: settle them either way.
  return auto ? pack(known) : compact(known);
}

/** After modules are switched on or off: the tiles of those switched off go, and those switched on
 *  get their default tiles. Nothing to do before the board is first changed (it's the default). */
export function withModules(saved: Placed[] | null, before: ModuleId[], after: ModuleId[], auto: boolean) {
  if (!saved) return null;
  let board = saved.filter((t) => after.includes(tileModule(t.id)));
  for (const [id] of defaultBoard) {
    const module = tileModule(id);
    if (after.includes(module) && !before.includes(module) && !board.some((t) => t.id === id)) {
      const { w, h } = newTile(id);
      board = add(board, id, w, h, auto);
    }
  }
  return auto ? pack(board) : board;
}

function placed(id: string, size: Size): Placed {
  const [w, h] = cells(size);
  return { id, w, h, x: 0, y: 0 };
}

type Rect = { left: number; top: number; width: number; height: number };
/** Where each tile was last drawn in edit mode. Outside the component, so it survives the board
 *  leaving the screen while the Add tiles list is open: the added tile then fades in. */
let lastRects = new Map<string, Rect>();

type BoardProps = {
  tiles: Placed[];
  open: (module: ModuleId) => void;
  editing: boolean;
  /** Auto-arrange (fill gaps) rather than free placement. */
  auto: boolean;
  onChange: (tiles: Placed[]) => void;
  /** A tile was held down: start edit mode. */
  onLongPress: () => void;
};

/** A tile being dragged. `tiles` is the board as it's shown: every step is worked out from `from`,
 *  the board before the drag, so tiles that made room spring back once the dragged one moves on. */
type Drag = {
  id: string;
  startX: number;
  startY: number;
  /** Where the pointer is now, and where in the tile it was grabbed (px): the tile follows it. */
  pointerX: number;
  pointerY: number;
  offsetX: number;
  offsetY: number;
  /** The grabbed cell within the tile. */
  grabX: number;
  grabY: number;
  x: number;
  y: number;
  moved: boolean;
  from: Placed[];
  tiles: Placed[];
};

/** The home board: each tile in its cells of the 4-column grid, drawn by its module. In edit mode
 *  the tiles stop working and can be dragged (or moved with the arrow keys), resized and removed. */
export function Board({ tiles, open, editing, auto, onChange, onLongPress }: BoardProps) {
  const board = useRef<HTMLDivElement>(null);
  // The drag lives in a ref, read by the pointer handlers: several pointer events can arrive before
  // React draws again, and each must see the last one's result. The state copy is for drawing.
  const dragging = useRef<Drag | null>(null);
  const [drag, setDragState] = useState<Drag | null>(null);
  const setDrag = (d: Drag | null) => {
    dragging.current = d;
    setDragState(d);
  };
  const press = useRef<{ timer: number; x: number; y: number } | null>(null);
  const swallowClick = useRef(false);

  // While dragging, the others make room live; the drop keeps what's shown.
  const shown = drag?.moved ? drag.tiles : tiles;

  const slot = (id: string) => board.current!.querySelector<HTMLElement>(`[data-id="${CSS.escape(id)}"]`);

  /** The dragged tile is drawn under the pointer, wherever its cell is: smooth, not cell by cell. */
  function follow() {
    const d = dragging.current;
    const el = d?.moved && slot(d.id);
    if (!d || !el) return;
    el.style.transform = "";
    const r = el.getBoundingClientRect();
    el.style.transform = `translate(${d.pointerX - d.offsetX - r.left}px, ${d.pointerY - d.offsetY - r.top}px)`;
  }

  // In edit mode, a tile that changed place glides there from where it was ("FLIP": measure before
  // and after, animate the difference), and a new one fades in. Only in answer to an edit, so the
  // panel does no extra work otherwise, and not at all if Windows' animation effects are off.
  useLayoutEffect(() => {
    follow(); // after React moved the dragged tile to its new cell
    if (!editing || reducedMotion()) return void (lastRects = new Map());
    const before = lastRects;
    lastRects = new Map();
    for (const el of board.current!.children as HTMLCollectionOf<HTMLElement>) {
      // Layout position, not getBoundingClientRect: that would include a glide still under way.
      const r = { left: el.offsetLeft, top: el.offsetTop, width: el.offsetWidth, height: el.offsetHeight };
      lastRects.set(el.dataset.id!, r);
      const old = before.get(el.dataset.id!);
      if (dragging.current?.moved && el.dataset.id === dragging.current.id) continue; // it follows the pointer
      if (!old) {
        // Not on entering edit mode, when nothing was measured yet: only a tile that's just been added.
        if (before.size > 0)
          el.animate([{ opacity: 0, transform: "scale(0.96)" }, {}], { duration: APPEAR_MS, easing: "ease-out" });
      } else if (old.left !== r.left || old.top !== r.top || old.width !== r.width || old.height !== r.height) {
        // A glide still under way starts the next from where the tile is drawn now, so it doesn't jump.
        const running = el.getAnimations();
        const now = running.length > 0 ? new DOMMatrix(getComputedStyle(el).transform) : new DOMMatrix();
        running.forEach((a) => a.cancel());
        const from = `translate(${old.left + now.e - r.left}px, ${old.top + now.f - r.top}px) scale(${old.width / r.width}, ${old.height / r.height})`;
        el.animate([{ transform: from, transformOrigin: "0 0" }, { transformOrigin: "0 0" }], { duration: MOVE_MS, easing: EASE });
      }
    }
  });

  /** The cell under the pointer, less where the tile was grabbed, so its top-left follows. */
  function cellAt(e: PointerEvent, grabX: number, grabY: number) {
    const rect = board.current!.getBoundingClientRect();
    const gap = parseFloat(getComputedStyle(board.current!).columnGap) || 0;
    const pitch = (rect.width + gap) / COLUMNS; // square cells: the same down as across
    return {
      x: Math.floor((e.clientX - rect.left) / pitch) - grabX,
      y: Math.floor((e.clientY - rect.top) / pitch) - grabY,
    };
  }

  function cancelPress() {
    if (press.current) clearTimeout(press.current.timer);
    press.current = null;
  }

  function pointerDown(e: PointerEvent<HTMLDivElement>, t: Placed) {
    if (e.button !== 0) return;
    if (!editing) {
      // Long-press to edit, but not on a slider: holding one is how it's dragged.
      if ((e.target as Element).closest("input")) return;
      press.current = {
        x: e.clientX,
        y: e.clientY,
        timer: window.setTimeout(() => {
          press.current = null;
          swallowClick.current = true; // the release isn't a click on the tile
          onLongPress();
        }, LONG_PRESS_MS),
      };
      return;
    }
    if ((e.target as Element).closest("button")) return; // the remove and resize badges
    // The board holds the pointer, not the tile: a tile's element moves in the page as it changes
    // place (to keep reading order), and moving an element drops its pointer capture.
    board.current!.setPointerCapture(e.pointerId);
    const grab = cellAt(e, t.x, t.y);
    const r = e.currentTarget.getBoundingClientRect();
    setDrag({
      id: t.id,
      startX: e.clientX,
      startY: e.clientY,
      pointerX: e.clientX,
      pointerY: e.clientY,
      offsetX: e.clientX - r.left,
      offsetY: e.clientY - r.top,
      grabX: grab.x,
      grabY: grab.y,
      x: t.x,
      y: t.y,
      moved: false,
      from: tiles,
      tiles,
    });
  }

  function pointerMove(e: PointerEvent) {
    if (press.current && Math.hypot(e.clientX - press.current.x, e.clientY - press.current.y) > DRAG_PX) cancelPress();
    const drag = dragging.current;
    if (!drag) return;
    const moved = drag.moved || Math.hypot(e.clientX - drag.startX, e.clientY - drag.startY) > DRAG_PX;
    const { x, y } = cellAt(e, drag.grabX, drag.grabY);
    const pointer = { pointerX: e.clientX, pointerY: e.clientY };
    if (moved === drag.moved && x === drag.x && y === drag.y) {
      dragging.current = { ...drag, ...pointer }; // no new cell: only the tile under the pointer moves
      return follow();
    }
    const next = moved && (x !== drag.x || y !== drag.y) ? move(drag.from, drag.id, x, y, auto) : drag.tiles;
    setDrag({ ...drag, ...pointer, moved, x, y, tiles: next });
  }

  function pointerUp() {
    cancelPress();
    const d = dragging.current;
    const el = d?.moved && slot(d.id);
    if (d && el) {
      // Glide from under the pointer into its cell, like any other tile that moved.
      const r = lastRects.get(d.id);
      const m = new DOMMatrix(getComputedStyle(el).transform);
      if (r) lastRects.set(d.id, { ...r, left: el.offsetLeft + m.e, top: el.offsetTop + m.f });
      el.style.transform = "";
      // Stays above its neighbours while it glides into its cell, not just while it's held.
      el.style.zIndex = "1";
      setTimeout(() => el.style.removeProperty("z-index"), MOVE_MS);
      // Free placement: once dropped, every tile rises as far as it can, the dropped one too.
      onChange(auto ? d.tiles : compact(d.tiles));
    }
    setDrag(null);
  }

  function cancelDrag() {
    cancelPress();
    const d = dragging.current;
    if (d) slot(d.id)?.style.removeProperty("transform");
    setDrag(null);
  }

  /** Removes a tile, after it fades out. */
  function removeTile(t: Placed) {
    const done = () => onChange(remove(tiles, t.id, auto));
    const el = slot(t.id);
    if (!el || reducedMotion()) return done();
    el.animate([{}, { opacity: 0, transform: "scale(0.9)" }], { duration: REMOVE_MS, easing: "ease-in", fill: "forwards" });
    // A timer, not the animation's finish event: that one waits for a frame, which a hidden page never draws.
    setTimeout(done, REMOVE_MS);
  }

  function keyDown(e: KeyboardEvent, t: Placed) {
    if (e.target !== e.currentTarget) return; // a badge's own keys
    const step = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] }[e.key];
    if (step) {
      e.preventDefault();
      const [dx, dy] = step;
      // With auto-arrange, left and right step through the order; "one cell left" may not change it.
      onChange(auto ? (dx !== 0 ? shift(tiles, t.id, dx as -1 | 1) : move(tiles, t.id, t.x, t.y + dy, true)) : compact(move(tiles, t.id, t.x + dx, t.y + dy, false)));
      // The tile's element stays the same (keyed by id), so it keeps the focus.
    } else if (e.key === "Delete" || e.key === "Backspace") {
      e.preventDefault();
      removeTile(t);
    }
  }

  function nextSize(t: Placed) {
    const { sizes } = info(t.id);
    const [w, h] = cells(sizes[(sizes.indexOf(sizeOf(t)) + 1) % sizes.length]);
    onChange(resize(tiles, t.id, w, h, auto));
  }

  return (
    <div
      ref={board}
      className={editing ? "board editing" : "board"}
      onPointerMove={pointerMove}
      onPointerUp={pointerUp}
      onPointerCancel={cancelDrag}
    >
      {shown.map((t) => {
        const module = tileModule(t.id);
        const { Tiles } = modules[module];
        const { name, sizes } = info(t.id);
        const size = sizeOf(t);
        const label = size.replace("x", "×");
        return (
          <div
            key={t.id}
            data-id={t.id}
            className={drag?.moved && drag.id === t.id ? "board-slot dragging" : "board-slot"}
            style={{ gridArea: `${t.y + 1} / ${t.x + 1} / span ${t.h} / span ${t.w}` }}
            tabIndex={editing ? 0 : undefined}
            role={editing ? "group" : undefined}
            aria-label={editing ? `${name}, ${label}. Arrow keys move it, Delete removes it.` : undefined}
            onPointerDown={(e) => pointerDown(e, t)}
            onPointerLeave={cancelPress}
            onClickCapture={(e) => {
              if (!swallowClick.current) return;
              swallowClick.current = false;
              e.stopPropagation();
              e.preventDefault();
            }}
            onKeyDown={editing ? (e) => keyDown(e, t) : undefined}
          >
            <div className="board-tile" inert={editing}>
              <Suspense fallback={null}>
                <Tiles kind={kindOf(t.id)} size={size} open={() => open(module)} />
              </Suspense>
            </div>
            {editing && (
              <>
                <button
                  className="tile-badge tile-remove"
                  aria-label={`Remove ${name}`}
                  title="Remove"
                  onClick={() => removeTile(t)}
                >
                  {MINUS}
                </button>
                {sizes.length > 1 && (
                  <button
                    className="tile-badge tile-resize"
                    aria-label={`Resize ${name}, now ${label}`}
                    title={`Size: ${label}`}
                    onClick={() => nextSize(t)}
                  >
                    {RESIZE}
                  </button>
                )}
              </>
            )}
          </div>
        );
      })}
    </div>
  );
}

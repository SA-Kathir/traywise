import { Suspense, useRef, useState, type KeyboardEvent, type PointerEvent } from "react";
import type { ModuleId } from "../bindings";
import { defaultBoard, modules, tileModule, type Size } from "../modules";
import { COLUMNS, move, pack, remove, resize, shift, type Placed } from "./board";

// Segoe Fluent Icons glyphs.
const MINUS = String.fromCodePoint(0xe738);
const RESIZE = String.fromCodePoint(0xe740); // "Full screen": two corners

/** Holding a tile this long (without moving) starts edit mode. */
const LONG_PRESS_MS = 500;
/** A pointer that moves this far (px) is a drag, not a click or a long-press. */
const DRAG_PX = 5;

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

function placed(id: string, size: Size): Placed {
  const [w, h] = cells(size);
  return { id, w, h, x: 0, y: 0 };
}

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

/** A tile being dragged. `tiles` is the board as it's shown, rebuilt from the last step rather than
 *  from before the drag, so a tile pushed aside stays put while the dragged one moves around. */
type Drag = {
  id: string;
  startX: number;
  startY: number;
  grabX: number;
  grabY: number;
  x: number;
  y: number;
  moved: boolean;
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
    setDrag({ id: t.id, startX: e.clientX, startY: e.clientY, grabX: grab.x, grabY: grab.y, x: t.x, y: t.y, moved: false, tiles });
  }

  function pointerMove(e: PointerEvent) {
    if (press.current && Math.hypot(e.clientX - press.current.x, e.clientY - press.current.y) > DRAG_PX) cancelPress();
    const drag = dragging.current;
    if (!drag) return;
    const moved = drag.moved || Math.hypot(e.clientX - drag.startX, e.clientY - drag.startY) > DRAG_PX;
    const { x, y } = cellAt(e, drag.grabX, drag.grabY);
    if (moved === drag.moved && x === drag.x && y === drag.y) return;
    const next = moved && (x !== drag.x || y !== drag.y) ? move(drag.tiles, drag.id, x, y, auto) : drag.tiles;
    setDrag({ ...drag, moved, x, y, tiles: next });
  }

  function pointerUp() {
    cancelPress();
    if (dragging.current?.moved) onChange(dragging.current.tiles);
    setDrag(null);
  }

  function keyDown(e: KeyboardEvent, t: Placed) {
    if (e.target !== e.currentTarget) return; // a badge's own keys
    const step = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] }[e.key];
    if (step) {
      e.preventDefault();
      const [dx, dy] = step;
      // With auto-arrange, left and right step through the order; "one cell left" may not change it.
      onChange(auto && dx !== 0 ? shift(tiles, t.id, dx as -1 | 1) : move(tiles, t.id, t.x + dx, t.y + dy, auto));
      // The tile's element stays the same (keyed by id), so it keeps the focus.
    } else if (e.key === "Delete" || e.key === "Backspace") {
      e.preventDefault();
      onChange(remove(tiles, t.id, auto));
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
      onPointerCancel={() => (cancelPress(), setDrag(null))}
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
                  onClick={() => onChange(remove(tiles, t.id, auto))}
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

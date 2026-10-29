import { Suspense } from "react";
import type { ModuleId } from "../bindings";
import { defaultBoard, modules, tileModule, type Size } from "../modules";
import { pack, type Placed } from "./board";

const cells = (size: Size) => size.split("x").map(Number) as [w: number, h: number];

/** A new user's board: the default tiles of the modules that are on. */
export function startingBoard(enabled: ModuleId[]): Placed[] {
  return pack(
    defaultBoard
      .filter(([id]) => enabled.includes(tileModule(id)))
      .map(([id, size]) => {
        const [w, h] = cells(size);
        return { id, w, h, x: 0, y: 0 };
      }),
  );
}

/** The home board: each tile in its cells of the 4-column grid, drawn by its module. */
export function Board({ tiles, open }: { tiles: Placed[]; open: (module: ModuleId) => void }) {
  return (
    <div className="board">
      {tiles.map((t) => {
        const module = tileModule(t.id);
        const { Tiles } = modules[module];
        return (
          <div
            key={t.id}
            className="board-slot"
            style={{ gridArea: `${t.y + 1} / ${t.x + 1} / span ${t.h} / span ${t.w}` }}
          >
            <Suspense fallback={null}>
              <Tiles kind={t.id.slice(module.length + 1)} size={`${t.w}x${t.h}` as Size} open={() => open(module)} />
            </Suspense>
          </div>
        );
      })}
    </div>
  );
}

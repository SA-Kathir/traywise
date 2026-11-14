import { modules, type Size } from "../modules";
import type { ModuleId } from "../bindings";

/** One thing the gallery can show: a tile (by id) at a size, with its name and a note on what it is. */
export type TileOption = { id: string; size: Size; name: string; note?: string };

/** The tile a kind belongs to: itself, or the tile it is a variant of. */
export const groupOf = (module: ModuleId, kind: string) => modules[module].tiles[kind].group ?? kind;

/** Every tile (by kind) in `group`: the base tile first, then its variants. */
export const kindsIn = (module: ModuleId, group: string) =>
  Object.entries(modules[module].tiles)
    .filter(([kind, t]) => (t.group ?? kind) === group)
    .sort(([a], [b]) => Number(b === group) - Number(a === group))
    .map(([kind]) => kind);

const cells = (size: Size) => size.split("x").map(Number) as [w: number, h: number];

/** Everything a tile and its variants come in, smallest first (a row, then wider, then a row taller). */
export function optionsOf(module: ModuleId, group: string): TileOption[] {
  return kindsIn(module, group)
    .flatMap((kind) => {
      const t = modules[module].tiles[kind];
      return t.sizes.map((size) => ({ id: `${module}/${kind}`, size, name: t.name, note: t.notes?.[size] }));
    })
    .sort((a, b) => cells(a.size)[1] - cells(b.size)[1] || cells(a.size)[0] - cells(b.size)[0]);
}

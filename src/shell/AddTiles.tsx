import { useState } from "react";
import type { ModuleId } from "../bindings";
import { modules, type Size } from "../modules";
import type { Placed } from "./board";
import { Num } from "./Tile";

// Segoe Fluent Icons glyphs.
const ADD = String.fromCodePoint(0xe710);
const CLOSE = String.fromCodePoint(0xe711);

const SIZES: Size[] = ["1x1", "2x1", "4x1", "2x2", "4x2"];
const sizeLabel = (size: Size) => size.replace("x", "×");

/** Edit mode's tray of tiles to add, over the board: those of modules that are on and not on the
 *  board yet, added at the size picked at the top. */
export function AddTiles({
  enabled,
  tiles,
  onAdd,
  onClose,
  openModules,
}: {
  enabled: ModuleId[];
  tiles: Placed[];
  onAdd: (id: string, size: Size) => void;
  onClose: () => void;
  openModules: () => void;
}) {
  const [size, setSize] = useState<Size>("2x1");
  const onBoard = new Set(tiles.map((t) => t.id));
  const rows = enabled.flatMap((id) =>
    Object.entries(modules[id].tiles)
      .filter(([kind]) => !onBoard.has(`${id}/${kind}`))
      .map(([kind, tile]) => ({ id: `${id}/${kind}`, module: modules[id].name, ...tile })),
  );
  return (
    <>
      <div className="scrim" onClick={onClose} />
      <section className="tray" role="dialog" aria-label="Add tiles">
        <div className="row">
          <h1 className="grow">Add tiles</h1>
          <button className="icon-button" aria-label="Close" title="Close" onClick={onClose}>
            {CLOSE}
          </button>
        </div>
        <div className="segmented sizes" role="radiogroup" aria-label="Size">
          {SIZES.map((s) => (
            <button key={s} role="radio" aria-checked={size === s} className={size === s ? "on" : ""} onClick={() => setSize(s)}>
              <Num text={sizeLabel(s)} />
            </button>
          ))}
        </div>
        {rows.length === 0 && <p className="hint">Every tile is on your board.</p>}
        {rows.map((t) => (
          <button key={t.id} className="link-row" disabled={!t.sizes.includes(size)} onClick={() => onAdd(t.id, size)}>
            <span className="grow">
              <span className="tile-label">{t.name}</span>
              <span className="module-desc">
                {t.module} · {t.sizes.map(sizeLabel).join(" ")}
              </span>
            </span>
            <span className="icon add" aria-hidden>
              {ADD}
            </span>
          </button>
        ))}
        <button className="link-row" onClick={openModules}>
          <span className="grow">Turn on more modules</span>
          <span className="icon" aria-hidden>
            {String.fromCodePoint(0xe76c)}
          </span>
        </button>
      </section>
    </>
  );
}

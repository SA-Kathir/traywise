import { useState } from "react";
import type { ModuleId } from "../bindings";
import { modules, type Size } from "../modules";
import type { Placed } from "./board";
import { Num } from "./Tile";

// Segoe Fluent Icons glyphs.
const ADD = String.fromCodePoint(0xe710);
const CLOSE = String.fromCodePoint(0xe711);
const BACK = String.fromCodePoint(0xe72b);

const sizeLabel = (size: Size) => size.replace("x", "×");

/** Edit mode's tray for adding tiles, module first: the modules that are on and have a tile not on
 *  the board yet, then (after picking one) its tiles, each with the sizes it comes in. */
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
  const [picked, setPicked] = useState<ModuleId | null>(null);
  const onBoard = new Set(tiles.map((t) => t.id));
  const free = (id: ModuleId) =>
    Object.entries(modules[id].tiles)
      .filter(([kind]) => !onBoard.has(`${id}/${kind}`))
      .map(([kind, tile]) => ({ id: `${id}/${kind}`, ...tile }));
  const available = enabled.filter((id) => free(id).length > 0);
  // Adding a module's last tile empties it: fall back to the list.
  const module = picked && available.includes(picked) ? picked : null;
  return (
    <>
      <div className="scrim" onClick={onClose} />
      <section className="tray" role="dialog" aria-label="Add tiles">
        <div className="row">
          {module && (
            <button className="icon-button" aria-label="Back" title="Back" onClick={() => setPicked(null)}>
              {BACK}
            </button>
          )}
          <h1 className="grow">{module ? modules[module].name : "Add tiles"}</h1>
          <button className="icon-button" aria-label="Close" title="Close" onClick={onClose}>
            {CLOSE}
          </button>
        </div>
        {module
          ? free(module).map((t) => (
              <div key={t.id} className="tray-tile">
                <span className="tile-label">{t.name}</span>
                <div className="segmented sizes" role="group" aria-label={`${t.name} size`}>
                  {t.sizes.map((s) => (
                    <button key={s} onClick={() => onAdd(t.id, s)}>
                      <Num text={sizeLabel(s)} />
                    </button>
                  ))}
                </div>
              </div>
            ))
          : available.map((id) => (
              <button key={id} className="link-row" onClick={() => setPicked(id)}>
                <span className="module-icon" aria-hidden>
                  {modules[id].icon}
                </span>
                <span className="grow">
                  <span className="tile-label">{modules[id].name}</span>
                  <span className="module-desc">{modules[id].description}</span>
                </span>
                <span className="icon add" aria-hidden>
                  {ADD}
                </span>
              </button>
            ))}
        {!module && available.length === 0 && <p className="hint">Every tile is on your board.</p>}
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

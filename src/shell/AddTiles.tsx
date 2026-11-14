import { useState, type ReactNode } from "react";
import type { ModuleId } from "../bindings";
import { defaultBoard, modules, tileModule, type Size } from "../modules";
import type { Placed } from "./board";
import { Gallery } from "./Gallery";
import { groupOf, kindsIn, optionsOf, type TileOption } from "./options";

// Segoe Fluent Icons glyphs.
const ADD = String.fromCodePoint(0xe710);
const CLOSE = String.fromCodePoint(0xe711);
const BACK = String.fromCodePoint(0xe72b);
const CHEVRON = String.fromCodePoint(0xe76c);

/** The tray both flows live in: a title, a back button when there is somewhere to go back to, a close button. */
function Tray({ title, onBack, onClose, label, children }: { title: string; onBack?: () => void; onClose: () => void; label: string; children: ReactNode }) {
  return (
    <>
      <div className="scrim" onClick={onClose} />
      <section className="tray" role="dialog" aria-label={label}>
        <div className="row">
          {onBack && (
            <button className="icon-button" aria-label="Back" title="Back" onClick={onBack}>
              {BACK}
            </button>
          )}
          <h1 className="grow">{title}</h1>
          <button className="icon-button" aria-label="Close" title="Close" onClick={onClose}>
            {CLOSE}
          </button>
        </div>
        {children}
      </section>
    </>
  );
}

/** Edit mode's tray for adding tiles, module first: the modules that are on and have a tile not on
 *  the board yet, then its tiles, then (after picking one) the tile as it looks, to swipe through its
 *  sizes and variants. A module with one tile left skips the list. */
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
  const [pickedTile, setPickedTile] = useState<string | null>(null);
  const onBoard = new Set(tiles.map((t) => t.id));
  /** The module's tiles not on the board yet (a tile and its variants count as one). */
  const free = (id: ModuleId) =>
    [...new Set(Object.entries(modules[id].tiles).map(([kind, t]) => t.group ?? kind))].filter(
      (group) => !kindsIn(id, group).some((kind) => onBoard.has(`${id}/${kind}`)),
    );
  const available = enabled.filter((id) => free(id).length > 0);
  // Adding a module's last tile empties it: fall back to the list.
  const module = picked && available.includes(picked) ? picked : null;
  const groups = module ? free(module) : [];
  const group = module && (pickedTile && groups.includes(pickedTile) ? pickedTile : groups.length === 1 ? groups[0] : null);
  const back = group && groups.length > 1 ? () => setPickedTile(null) : module ? () => (setPicked(null), setPickedTile(null)) : undefined;

  if (module && group) {
    const options = optionsOf(module, group);
    // Opens on the tile's size on the default board (or its smallest).
    const first = defaultBoard.find(([id]) => id === `${module}/${group}`)?.[1];
    const start = Math.max(0, options.findIndex((o) => o.id === `${module}/${group}` && o.size === first));
    return (
      <Tray title={modules[module].tiles[group].name} onBack={back} onClose={onClose} label="Add tiles">
        <Gallery options={options} start={start} action="Add" onPick={({ id, size }) => onAdd(id, size)} />
      </Tray>
    );
  }
  return (
    <Tray title={module ? modules[module].name : "Add tiles"} onBack={back} onClose={onClose} label="Add tiles">
      {module
        ? groups.map((g) => (
            <button key={g} className="link-row" onClick={() => setPickedTile(g)}>
              <span className="grow">{modules[module].tiles[g].name}</span>
              <span className="icon" aria-hidden>
                {CHEVRON}
              </span>
            </button>
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
          {CHEVRON}
        </span>
      </button>
    </Tray>
  );
}

/** The resize button's tray: the same gallery, for one tile on the board, opening on what it is now. */
export function ChangeTile({ tile, onPick, onClose }: { tile: Placed; onPick: (o: TileOption) => void; onClose: () => void }) {
  const module = tileModule(tile.id);
  const group = groupOf(module, tile.id.slice(tile.id.indexOf("/") + 1));
  const options = optionsOf(module, group);
  const now = options.findIndex((o) => o.id === tile.id && o.size === `${tile.w}x${tile.h}`);
  return (
    <Tray title={`Change ${modules[module].tiles[group].name}`} onClose={onClose} label="Change tile">
      <Gallery options={options} start={Math.max(0, now)} current={now < 0 ? undefined : now} action="Apply" onPick={onPick} />
    </Tray>
  );
}

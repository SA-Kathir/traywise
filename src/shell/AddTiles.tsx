import type { ModuleId } from "../bindings";
import { modules } from "../modules";
import type { Placed } from "./board";

// Segoe Fluent Icons glyph.
const ADD = String.fromCodePoint(0xe710);

/** Edit mode's list of tiles to add: those of modules that are on and not on the board yet. */
export function AddTiles({
  enabled,
  tiles,
  onAdd,
  openModules,
}: {
  enabled: ModuleId[];
  tiles: Placed[];
  onAdd: (id: string) => void;
  openModules: () => void;
}) {
  const onBoard = new Set(tiles.map((t) => t.id));
  const groups = enabled
    .map((id) => [id, Object.entries(modules[id].tiles).filter(([kind]) => !onBoard.has(`${id}/${kind}`))] as const)
    .filter(([, kinds]) => kinds.length > 0);
  return (
    <>
      {groups.length === 0 && <p className="hint">Every tile is on your board.</p>}
      {groups.map(([id, kinds]) => (
        <section key={id}>
          <h2>{modules[id].name}</h2>
          {kinds.map(([kind, { name }]) => (
            <button key={kind} className="link-row" onClick={() => onAdd(`${id}/${kind}`)}>
              <span className="grow">{name}</span>
              <span className="icon" aria-hidden>
                {ADD}
              </span>
            </button>
          ))}
        </section>
      ))}
      <section>
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

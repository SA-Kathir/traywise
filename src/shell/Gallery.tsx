import { Suspense, useState, type CSSProperties, type KeyboardEvent } from "react";
import { modules, tileModule, type Size } from "../modules";
import type { TileOption } from "./options";
import { Pending } from "./useLive";

// Segoe Fluent Icons glyphs.
const PREV = String.fromCodePoint(0xe76b);
const NEXT = String.fromCodePoint(0xe76c);

const CELL = 76;
const GAP = 8;
/** The room for a tile in the stage (px): the tray's width less the two arrows. */
const ROOM = 236;
const cells = (size: Size) => size.split("x").map(Number) as [w: number, h: number];
const px = (n: number) => n * CELL + (n - 1) * GAP;

/** The Add tiles and Change tray's picture book: the tile as it really looks (live, at its own size,
 *  shrunk only to fit), with arrows to swipe through everything it comes in. `current`: the one
 *  already on the board. `action` is the button's word. */
export function Gallery({
  options,
  start,
  current,
  action,
  onPick,
}: {
  options: TileOption[];
  start: number;
  current?: number;
  action: string;
  onPick: (o: TileOption) => void;
}) {
  const [i, setI] = useState(start);
  const o = options[i];
  const [w, h] = cells(o.size);
  const { Tiles } = modules[tileModule(o.id)];
  const fit = Math.min(1, ROOM / px(w));
  const step = (by: number) => setI((i + by + options.length) % options.length);
  const keys = (e: KeyboardEvent) => {
    if (e.key === "ArrowLeft") step(-1);
    else if (e.key === "ArrowRight") step(1);
  };
  return (
    <div className="gallery" onKeyDown={keys}>
      <div className="gallery-stage">
        <button className="icon-button" aria-label="Previous" title="Previous" disabled={options.length < 2} onClick={() => step(-1)}>
          {PREV}
        </button>
        <div className="gallery-fit" style={{ width: px(w) * fit, height: px(h) * fit }}>
          <div
            key={`${o.id}/${o.size}`}
            className="board gallery-board"
            style={{ "--fit": fit, gridTemplateColumns: `repeat(${w}, ${CELL}px)`, gridAutoRows: `${CELL}px` } as CSSProperties}
            inert
          >
            <div className="board-slot" style={{ gridArea: `1 / 1 / span ${h} / span ${w}` }}>
              <div className="board-tile">
                <Suspense fallback={<Pending />}>
                  <Tiles kind={o.id.slice(o.id.indexOf("/") + 1)} size={o.size} open={() => {}} />
                </Suspense>
              </div>
            </div>
          </div>
        </div>
        <button className="icon-button" aria-label="Next" title="Next" disabled={options.length < 2} onClick={() => step(1)}>
          {NEXT}
        </button>
        {i === current && <span className="gallery-current">Current</span>}
      </div>
      <div className="gallery-dots" aria-hidden>
        {options.map((_, j) => (
          <i key={j} className={j === i ? "on" : undefined} />
        ))}
      </div>
      <p className="gallery-name">{o.name}</p>
      <p className="gallery-note">{[o.size.replace("x", "×"), o.note].filter(Boolean).join(" · ")}</p>
      <button className="gallery-action" disabled={i === current} onClick={() => onPick(o)}>
        {action}
      </button>
    </div>
  );
}

import { useRef, useState, type KeyboardEvent, type PointerEvent } from "react";
import type { Placed } from "./board";
import { Num } from "./Tile";

// Segoe Fluent Icons glyphs.
const ADD = String.fromCodePoint(0xe710);
const CLOSE = String.fromCodePoint(0xe711);

/** A pointer that moves this far (px) is a drag, not a click. */
const DRAG_PX = 5;

/** Edit mode's overview of the home board's pages, over the board: each page as a small outline of
 *  where its tiles are (plain boxes, not the tiles themselves, so it costs next to nothing). Click
 *  one to go there; drag it, or use ←/→, to reorder; × (or Delete) removes it, asking first if it
 *  has tiles, which go back to Add tiles; + adds an empty page at the end. */
export function PagesOverview({
  pages,
  current,
  canAdd,
  onGo,
  onChange,
  onClose,
}: {
  pages: Placed[][];
  current: number;
  canAdd: boolean;
  onGo: (page: number) => void;
  /** New pages, and the page to show afterwards. */
  onChange: (pages: Placed[][], show: number) => void;
  onClose: () => void;
}) {
  // While a page is dragged: the order shown (indices into `pages`), and which one is held.
  const [order, setOrder] = useState<number[] | null>(null);
  const drag = useRef<{ from: number; x: number; y: number; moved: boolean } | null>(null);
  const [confirm, setConfirm] = useState<number | null>(null);
  const shown = order ?? pages.map((_, i) => i);

  /** Saves `next` (indices into `pages`), keeping the page that was on screen on screen. */
  const reorder = (next: number[]) => onChange(next.map((i) => pages[i]), next.indexOf(current));

  function remove(i: number) {
    if (pages[i].length > 0 && confirm !== i) return setConfirm(i);
    setConfirm(null);
    // The page on screen stays; if it's the one removed, the next takes its place (or the one before, at the end).
    onChange(pages.filter((_, j) => j !== i), i < current ? current - 1 : Math.min(current, pages.length - 2));
  }

  function pointerDown(e: PointerEvent<HTMLElement>, i: number) {
    if (e.button !== 0 || (e.target as Element).closest(".page-remove")) return;
    e.currentTarget.parentElement!.setPointerCapture(e.pointerId);
    drag.current = { from: i, x: e.clientX, y: e.clientY, moved: false };
  }

  function pointerMove(e: PointerEvent) {
    const d = drag.current;
    if (!d) return;
    d.moved ||= Math.hypot(e.clientX - d.x, e.clientY - d.y) > DRAG_PX;
    if (!d.moved) return;
    // The thumbnail under the pointer: the held page takes its place.
    const over = document.elementsFromPoint(e.clientX, e.clientY).find((el): el is HTMLElement => el instanceof HTMLElement && el.dataset.thumb !== undefined);
    const now = order ?? pages.map((_, i) => i);
    const to = over ? now.indexOf(Number(over.dataset.thumb)) : -1;
    if (to < 0 || now[to] === d.from) return setOrder(now);
    const next = now.filter((i) => i !== d.from);
    next.splice(to, 0, d.from);
    setOrder(next);
  }

  function pointerUp() {
    const d = drag.current;
    drag.current = null;
    if (d && !d.moved) onGo(d.from);
    else if (order) reorder(order);
    setOrder(null);
  }

  function keyDown(e: KeyboardEvent, i: number) {
    const by = { ArrowLeft: -1, ArrowRight: 1 }[e.key];
    const at = shown.indexOf(i);
    if (by && !e.ctrlKey && shown[at + by] !== undefined) {
      e.preventDefault();
      const next = [...shown];
      [next[at], next[at + by]] = [next[at + by], next[at]];
      reorder(next);
    } else if (e.key === "Delete") {
      e.preventDefault();
      if (pages.length > 1) remove(i);
    } else if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      onGo(i);
    }
  }

  return (
    <>
      <div className="scrim" onClick={onClose} />
      <section className="tray" role="dialog" aria-label="Pages">
        <div className="row">
          <h1 className="grow">Pages</h1>
          <button className="icon-button" aria-label="Close" title="Close" onClick={onClose}>
            {CLOSE}
          </button>
        </div>
        <p className="hint">Drag to reorder. Removing a page puts its tiles back in Add tiles.</p>
        <div className="thumbs" onPointerMove={pointerMove} onPointerUp={pointerUp} onPointerCancel={() => ((drag.current = null), setOrder(null))}>
          {shown.map((i, at) => (
            <div
              key={i}
              data-thumb={i}
              className={`thumb${i === current ? " on" : ""}${order && drag.current?.from === i ? " held" : ""}`}
              tabIndex={0}
              role="button"
              aria-label={`Page ${at + 1}, ${pages[i].length} tiles${i === current ? ", on screen" : ""}. Enter goes there, arrow keys move it, Delete removes it.`}
              onPointerDown={(e) => pointerDown(e, i)}
              onKeyDown={(e) => keyDown(e, i)}
            >
              <div className="thumb-grid" aria-hidden>
                {pages[i].map((t) => (
                  <span key={t.id} style={{ gridArea: `${t.y + 1} / ${t.x + 1} / span ${t.h} / span ${t.w}` }} />
                ))}
              </div>
              <Num text={String(at + 1)} />
              {pages.length > 1 && (
                <button
                  className={confirm === i ? "page-remove on" : "page-remove"}
                  aria-label={confirm === i ? `Remove page ${at + 1} and its tiles` : `Remove page ${at + 1}`}
                  title="Remove page"
                  onBlur={() => setConfirm(null)}
                  onClick={() => remove(i)}
                >
                  {confirm === i ? "Remove?" : CLOSE}
                </button>
              )}
            </div>
          ))}
          {canAdd && (
            <button className="thumb thumb-add" aria-label="Add a page" title="Add a page" onClick={() => onChange([...pages, []], current)}>
              {ADD}
            </button>
          )}
        </div>
      </section>
    </>
  );
}

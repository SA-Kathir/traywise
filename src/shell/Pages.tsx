import { useEffect, useLayoutEffect, useRef, type PointerEvent, type ReactNode, type WheelEvent } from "react";

/** A sideways drag this far (px), and clearly more across than down, turns the page. */
const SWIPE_PX = 40;
/** Wheel movement (px) that turns the page; a touchpad sends many small steps. */
const WHEEL_PX = 40;
/** Wheel events closer together than this (ms) are one gesture: it turns one page, not several. */
const WHEEL_GAP_MS = 250;
const SLIDE_MS = 180;
const EASE = "cubic-bezier(0.2, 0.8, 0.2, 1)";
const reducedMotion = () => matchMedia("(prefers-reduced-motion: reduce)").matches;

/** The home board's pages: only the one on screen is drawn (`children`). The others are reached by
 *  dragging sideways (`swipe`; off in edit mode, where dragging moves tiles), a sideways wheel (a
 *  touchpad, or Shift with a mouse wheel), Ctrl+←/→, or the dots underneath. */
export function Pages({
  count,
  page,
  onPage,
  swipe,
  children,
}: {
  count: number;
  page: number;
  onPage: (page: number) => void;
  swipe: boolean;
  children: ReactNode;
}) {
  const box = useRef<HTMLDivElement>(null);
  const shown = useRef(page);
  const start = useRef<{ x: number; y: number; done: boolean } | null>(null);
  const wheel = useRef({ sum: 0, last: -Infinity, turned: false });
  const swallowClick = useRef(false);

  const turn = (by: number) => {
    const next = page + by;
    if (next >= 0 && next < count) onPage(next);
  };

  // The new page slides in from the side it's on. Only in answer to the user turning the page.
  useLayoutEffect(() => {
    const by = Math.sign(page - shown.current);
    shown.current = page;
    if (by === 0 || reducedMotion()) return;
    box.current!.animate([{ transform: `translateX(${by * 32}px)`, opacity: 0 }, {}], { duration: SLIDE_MS, easing: EASE });
  }, [page]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const by = { ArrowLeft: -1, ArrowRight: 1 }[e.key];
      if (!by || !e.ctrlKey || (e.target as Element).closest("input, textarea")) return;
      e.preventDefault();
      turn(by);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  function onWheel(e: WheelEvent) {
    const dx = e.shiftKey ? e.deltaY || e.deltaX : Math.abs(e.deltaX) > Math.abs(e.deltaY) ? e.deltaX : 0;
    const w = wheel.current;
    if (e.timeStamp - w.last > WHEEL_GAP_MS) Object.assign(w, { sum: 0, turned: false });
    w.last = e.timeStamp;
    if (dx === 0 || w.turned) return;
    w.sum += dx;
    if (Math.abs(w.sum) < WHEEL_PX) return;
    w.turned = true;
    turn(Math.sign(w.sum));
  }

  function onPointerDown(e: PointerEvent) {
    // Not on a slider: dragging one sideways is how it's set.
    swallowClick.current = false;
    const ok = swipe && e.button === 0 && !(e.target as Element).closest("input");
    start.current = ok ? { x: e.clientX, y: e.clientY, done: false } : null;
  }

  function onPointerMove(e: PointerEvent) {
    const s = start.current;
    if (!s || s.done) return;
    const dx = e.clientX - s.x;
    if (Math.abs(dx) < SWIPE_PX || Math.abs(dx) < 2 * Math.abs(e.clientY - s.y)) return;
    s.done = true;
    swallowClick.current = true; // the release isn't a click on the tile it started on
    turn(dx < 0 ? 1 : -1);
  }

  return (
    <>
      <div
        ref={box}
        className="pages"
        onWheel={onWheel}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={() => (start.current = null)}
        onPointerCancel={() => (start.current = null)}
        onClickCapture={(e) => {
          if (!swallowClick.current) return;
          swallowClick.current = false;
          e.stopPropagation();
          e.preventDefault();
        }}
      >
        {children}
      </div>
      {count > 1 && (
        <nav className="page-dots" aria-label="Pages">
          {Array.from({ length: count }, (_, i) => (
            <button
              key={i}
              aria-label={`Page ${i + 1}`}
              title={`Page ${i + 1}`}
              aria-current={i === page ? "page" : undefined}
              onClick={() => onPage(i)}
            />
          ))}
        </nav>
      )}
    </>
  );
}

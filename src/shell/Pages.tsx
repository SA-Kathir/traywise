import { useEffect, useLayoutEffect, useRef, type ReactNode } from "react";

const reducedMotion = () => matchMedia("(prefers-reduced-motion: reduce)").matches;

/** The home board's pages, side by side in a row that scrolls sideways and snaps to each page. The
 *  scrolling is the browser's own, so a touchpad, a touchscreen or Shift with the mouse wheel moves
 *  the pages under the fingers, smoothly; Ctrl+←/→, PageUp/PageDown and the dots underneath scroll
 *  to a page. Only the page on screen and its neighbours are drawn (`render`), so the next one is
 *  ready as it slides in and the rest cost nothing. */
export function Pages({
  count,
  page,
  onPage,
  render,
}: {
  count: number;
  page: number;
  onPage: (page: number) => void;
  render: (page: number) => ReactNode;
}) {
  const row = useRef<HTMLDivElement>(null);

  /** The page the row is scrolled to (or nearest to). */
  const shown = () => Math.round(row.current!.scrollLeft / row.current!.clientWidth);
  const go = (to: number) => {
    const el = row.current!;
    if (to < 0 || to >= count) return;
    el.scrollTo({ left: to * el.clientWidth, behavior: reducedMotion() ? "instant" : "smooth" });
  };

  // A page picked from outside (opening the panel, adding or removing a page) is shown at once.
  useLayoutEffect(() => {
    const el = row.current!;
    if (shown() !== page) el.scrollTo({ left: page * el.clientWidth, behavior: "instant" });
  }, [page, count]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const arrow = e.ctrlKey && { ArrowLeft: -1, ArrowRight: 1 }[e.key];
      const by = arrow || { PageUp: -1, PageDown: 1 }[e.key];
      if (!by || (e.target as Element).closest("input, textarea")) return;
      e.preventDefault();
      go(shown() + by);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  return (
    <>
      <div
        ref={row}
        className="pages"
        // Once the row comes to rest on a page, that's the page on screen.
        onScrollEnd={() => shown() !== page && onPage(shown())}
      >
        {Array.from({ length: count }, (_, i) => (
          <div key={i} className="page" aria-hidden={i !== page || undefined} inert={i !== page}>
            {Math.abs(i - page) <= 1 && render(i)}
          </div>
        ))}
      </div>
      {count > 1 && (
        <nav className="page-dots" aria-label="Pages">
          {Array.from({ length: count }, (_, i) => (
            <button
              key={i}
              aria-label={`Page ${i + 1}`}
              title={`Page ${i + 1}`}
              aria-current={i === page ? "page" : undefined}
              onClick={() => go(i)}
            />
          ))}
        </nav>
      )}
    </>
  );
}

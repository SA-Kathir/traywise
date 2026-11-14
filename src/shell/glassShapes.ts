import { commands } from "../bindings";

/** The Glass look's Clear variant has no blur behind the panel, only under its tiles, the header pill and
 *  the tab bar. This tells Rust where those are, and again whenever they move, fade or scroll. Nothing
 *  runs while nothing moves; during the open animation it follows every frame, then stops. */

const PIECES = ".tile, section, .stats, .tabs, .page-dots, .panel > header";

/** How visible an element is: its own opacity times its parents'. */
function visibility(el: Element | null): number {
  let v = 1;
  for (; el && el !== document.documentElement; el = el.parentElement) {
    v *= Number(getComputedStyle(el).opacity);
    if (v === 0) break;
  }
  return v;
}

function report() {
  const w = window.innerWidth;
  const h = window.innerHeight;
  const shapes = [];
  for (const el of document.querySelectorAll<HTMLElement>(PIECES)) {
    const opacity = visibility(el);
    if (opacity < 0.02) continue;
    const clip = el.closest<HTMLElement>(".scroll")?.getBoundingClientRect();
    const r = el.getBoundingClientRect();
    const left = Math.max(r.left, clip?.left ?? 0, 0);
    const top = Math.max(r.top, clip?.top ?? 0, 0);
    const right = Math.min(r.right, clip?.right ?? w, w);
    const bottom = Math.min(r.bottom, clip?.bottom ?? h, h);
    if (right - left < 1 || bottom - top < 1) continue;
    const radius = parseFloat(getComputedStyle(el).borderTopLeftRadius) || 0;
    shapes.push({ x: left, y: top, width: right - left, height: bottom - top, radius, opacity });
  }
  void commands.glassShapes(shapes);
}

/** Starts following the pieces; returns the function that stops it and removes the blur. */
export function trackGlassShapes(): () => void {
  let queued = 0;
  const update = () => {
    cancelAnimationFrame(queued);
    queued = requestAnimationFrame(() => {
      report();
      // While anything animates (the tiles coming in, a page sliding), keep following it.
      if (document.getAnimations().some((a) => a.playState === "running")) update();
    });
  };
  const watch = new MutationObserver(update);
  watch.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ["class", "style", "data-island"] });
  window.addEventListener("resize", update);
  // The panel opened: the window may have a new size or place.
  window.addEventListener("tw-opened", update);
  document.addEventListener("scroll", update, { capture: true, passive: true });
  document.addEventListener("animationstart", update, true);
  document.addEventListener("transitionstart", update, true);
  update();
  return () => {
    cancelAnimationFrame(queued);
    watch.disconnect();
    window.removeEventListener("resize", update);
    window.removeEventListener("tw-opened", update);
    document.removeEventListener("scroll", update, true);
    document.removeEventListener("animationstart", update, true);
    document.removeEventListener("transitionstart", update, true);
    void commands.glassShapes([]);
  };
}

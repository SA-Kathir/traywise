/** The Glass look's clear tiles. The panel's tint is one layer under the whole page, so a tile on top of it
 *  was a grey sheet. This cuts a hole in the tint under every card, so each card shows the plain blur
 *  of the desktop, with only its own thin sheet and rim on top. The hole follows the card: its place, how
 *  see-through it is (the tiles fade in), and where a scrolling page clips it. Nothing runs while nothing
 *  moves; during the open and close animation it follows every frame, then stops. */

const CARDS = ".tile, section, .stats";
const RADIUS = 24;
/** How much of the tint the hole takes out: all of it over a dark desktop, less over a bright one
 *  (`--backdrop`, 0 to 1, from the brightness sample taken as the panel opens), so white text keeps some
 *  ground. */
const MAX_RESTORED = 0.5;

/** How visible an element is: its own opacity times its parents'. */
function visibility(el: Element | null): number {
  let v = 1;
  for (; el && el !== document.documentElement; el = el.parentElement) {
    v *= Number(getComputedStyle(el).opacity);
    if (v === 0) break;
  }
  return v;
}

function draw(root: HTMLElement) {
  const w = window.innerWidth;
  const h = window.innerHeight;
  let holes = "";
  for (const card of document.querySelectorAll<HTMLElement>(CARDS)) {
    const clip = card.closest<HTMLElement>(".scroll")?.getBoundingClientRect();
    const r = card.getBoundingClientRect();
    const left = Math.max(r.left, clip?.left ?? 0);
    const top = Math.max(r.top, clip?.top ?? 0);
    const right = Math.min(r.right, clip?.right ?? w);
    const bottom = Math.min(r.bottom, clip?.bottom ?? h);
    if (right - left < 1 || bottom - top < 1) continue;
    // White keeps the tint, black cuts it out completely, grey in between while a tile fades in.
    const brightness = Number(getComputedStyle(root).getPropertyValue("--backdrop")) || 0;
    const grey = Math.round(255 * (1 - (1 - MAX_RESTORED * brightness) * visibility(card)));
    const rx = Math.min(RADIUS, (right - left) / 2, (bottom - top) / 2);
    holes += `<rect x="${left}" y="${top}" width="${right - left}" height="${bottom - top}" rx="${rx}" fill="rgb(${grey},${grey},${grey})"/>`;
  }
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}"><rect width="${w}" height="${h}" fill="#fff"/>${holes}</svg>`;
  root.style.setProperty("--tw-glass-holes", `url("data:image/svg+xml,${encodeURIComponent(svg)}")`);
}

/** Starts following the cards; returns the function that stops it and puts the full tint back. */
export function trackGlassHoles(): () => void {
  const root = document.documentElement;
  let queued = 0;
  const update = () => {
    cancelAnimationFrame(queued);
    queued = requestAnimationFrame(() => {
      draw(root);
      // While anything animates (the tiles coming in), keep following it.
      if (document.getAnimations().some((a) => a.playState === "running")) update();
    });
  };
  const watch = new MutationObserver(update);
  watch.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ["class", "style", "data-island"] });
  window.addEventListener("resize", update);
  // The panel just measured the desktop behind it (App.tsx).
  window.addEventListener("tw-backdrop", update);
  // Scrolling doesn't bubble, but it can be caught on the way down.
  document.addEventListener("scroll", update, { capture: true, passive: true });
  document.addEventListener("animationstart", update, true);
  document.addEventListener("transitionstart", update, true);
  update();
  return () => {
    cancelAnimationFrame(queued);
    watch.disconnect();
    window.removeEventListener("resize", update);
    window.removeEventListener("tw-backdrop", update);
    document.removeEventListener("scroll", update, true);
    document.removeEventListener("animationstart", update, true);
    document.removeEventListener("transitionstart", update, true);
    root.style.removeProperty("--tw-glass-holes");
  };
}

import { useEffect, useRef } from "react";
import { events, type Motion, type Preset } from "../bindings";

/** After each open the background drifts this long, then slows to a stop over SLOW_MS.
 *  Stopping matters: continuous motion costs ~35% of a CPU core. */
const MOVE_MS = 10000;
const SLOW_MS = 2500;

/** The colour blobs the glass sits on, plus the SVG filter the cards use for edge refraction. */
export function LiquidBackground({ preset, motion }: { preset: Preset; motion: Motion }) {
  const bg = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // The blobs' CSS animations, driven directly so their speed can be eased down to zero.
    const drifts = bg.current?.getAnimations({ subtree: true }) ?? [];
    if (motion === "still") return;
    let timer: number | undefined;
    let frame = 0;
    const unlisten = events.panelShown.listen(() => {
      clearTimeout(timer);
      cancelAnimationFrame(frame);
      drifts.forEach((d) => {
        d.playbackRate = 1;
        d.play();
      });
      timer = window.setTimeout(() => {
        const start = performance.now();
        const slowDown = (now: number) => {
          const rate = 1 - (now - start) / SLOW_MS;
          if (rate <= 0) return drifts.forEach((d) => d.pause());
          drifts.forEach((d) => (d.playbackRate = rate));
          frame = requestAnimationFrame(slowDown);
        };
        frame = requestAnimationFrame(slowDown);
      }, MOVE_MS);
    });
    return () => {
      clearTimeout(timer);
      cancelAnimationFrame(frame);
      drifts.forEach((d) => d.pause()); // switched to still mid-drift
      unlisten.then((stop) => stop());
    };
  }, [motion]);

  return (
    <>
      <div ref={bg} className={`liquid-bg bg-${preset}`} aria-hidden>
        <span className="blob b1" />
        <span className="blob b2" />
        <span className="blob b3" />
      </div>
      <svg className="defs" aria-hidden>
        <filter id="liquid-refract" x="0" y="0" width="100%" height="100%">
          <feTurbulence type="fractalNoise" baseFrequency="0.012" numOctaves="2" seed="7" result="noise" />
          <feDisplacementMap in="SourceGraphic" in2="noise" scale="14" xChannelSelector="R" yChannelSelector="G" />
        </filter>
      </svg>
    </>
  );
}

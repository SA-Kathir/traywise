// The ring of dots on the Keep awake and Focus 2×2 tiles, which empties as a timer runs out.

export const RING = 24;

/** 24 dots in a circle; the first `lit` (clockwise from the top) are lit. */
export function Ring({ lit }: { lit: number }) {
  return (
    <svg className="awake-ring" viewBox="0 0 64 64" aria-hidden>
      {Array.from({ length: RING }, (_, i) => {
        const a = (i / RING) * 2 * Math.PI - Math.PI / 2;
        return <circle key={i} cx={32 + 27 * Math.cos(a)} cy={32 + 27 * Math.sin(a)} r={2.6} className={i < lit ? "lit" : undefined} />;
      })}
    </svg>
  );
}

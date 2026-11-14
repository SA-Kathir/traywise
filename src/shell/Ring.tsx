// The ring of dots on the Keep awake and Focus 2×2 tiles, which empties as a timer runs out.

export const RING = 24;

/** 24 dots in a circle; the first `lit` (clockwise from the top) are lit. */
export function Ring({ lit }: { lit: number }) {
  return (
    <svg className="awake-ring" viewBox="0 0 64 64" aria-hidden>
      <g className="dots">
        {Array.from({ length: RING }, (_, i) => {
          const a = (i / RING) * 2 * Math.PI - Math.PI / 2;
          return <circle key={i} cx={32 + 27 * Math.cos(a)} cy={32 + 27 * Math.sin(a)} r={2.6} className={i < lit ? "lit" : undefined} />;
        })}
      </g>
      {/* The Glass look draws a smooth ring instead of the dots. */}
      <g className="smooth" transform="rotate(-90 32 32)">
        <circle className="smooth-track" cx={32} cy={32} r={27} pathLength={100} />
        {lit > 0 && <circle className="smooth-fill" cx={32} cy={32} r={27} pathLength={100} strokeDasharray={`${(lit / RING) * 100} 100`} />}
      </g>
    </svg>
  );
}

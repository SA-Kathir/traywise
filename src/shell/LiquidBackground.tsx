/** The soft colour blobs the glass sits on, in the preset's colours (set on the panel). They don't
 *  move: blurred behind the glass they read as an even two-tone wash, and still costs no CPU. */
export function LiquidBackground() {
  return (
    <div className="liquid-bg" aria-hidden>
      <span className="blob b1" />
      <span className="blob b2" />
      <span className="blob b3" />
    </div>
  );
}

import { useEffect, useRef, useState, type CSSProperties } from "react";

/** At most one change per frame (~60 a second) while a slider is dragged. */
const SEND_EVERY_MS = 16;

/** A 0–100 slider that sends at most one change per frame while dragged (volume, brightness).
 *  `name` is read out as "<name> <what>"; `bare`: the slider alone, without its number. */
export function Slider({
  name,
  what = "volume",
  value,
  onChange,
  bare,
}: {
  name: string;
  what?: string;
  value: number;
  onChange: (v: number) => void;
  bare?: boolean;
}) {
  // While dragging, show the thumb where the pointer is, not the slightly older value Windows echoes back.
  const [dragged, setDragged] = useState<number | null>(null);
  const dragging = useRef(false);
  const lastSent = useRef(0);
  const pending = useRef<number | null>(null);
  const timer = useRef<number | undefined>(undefined);

  useEffect(() => () => clearTimeout(timer.current), []);

  function send(v: number) {
    const wait = SEND_EVERY_MS - (performance.now() - lastSent.current);
    if (wait <= 0) {
      lastSent.current = performance.now();
      onChange(v);
      return;
    }
    // Too soon: send the latest value once the frame is up, so the final position always lands.
    pending.current = v;
    timer.current ??= window.setTimeout(() => {
      timer.current = undefined;
      lastSent.current = performance.now();
      if (pending.current !== null) onChange(pending.current);
    }, wait);
  }

  const shown = dragged ?? value;
  const endDrag = () => {
    dragging.current = false;
    setDragged(null);
  };
  return (
    <>
      <input
        type="range"
        min={0}
        max={100}
        value={shown}
        style={{ "--fill": `${shown}%` } as CSSProperties}
        aria-label={`${name} ${what}`}
        onPointerDown={() => (dragging.current = true)}
        onPointerUp={endDrag}
        onPointerCancel={endDrag}
        onChange={(e) => {
          const v = Number(e.target.value);
          if (dragging.current) setDragged(v);
          send(v);
        }}
      />
      {!bare && <span className="mixer-value">{shown}</span>}
    </>
  );
}

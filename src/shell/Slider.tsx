import { useCallback, useEffect, useRef, useState, type CSSProperties } from "react";

/** At most one change per frame (~60 a second) while a slider is dragged. */
const SEND_EVERY_MS = 16;

/** `onChange` at most once per frame; when it is called too soon, the latest value goes out once
 *  the frame is up, so the final position always lands. Shared by the sliders and the dials. */
export function useSend(onChange: (v: number) => void) {
  const lastSent = useRef(0);
  const pending = useRef<number | null>(null);
  const timer = useRef<number | undefined>(undefined);
  const change = useRef(onChange);
  change.current = onChange;

  useEffect(() => () => clearTimeout(timer.current), []);

  return useCallback((v: number) => {
    const wait = SEND_EVERY_MS - (performance.now() - lastSent.current);
    if (wait <= 0) {
      lastSent.current = performance.now();
      change.current(v);
      return;
    }
    pending.current = v;
    timer.current ??= window.setTimeout(() => {
      timer.current = undefined;
      lastSent.current = performance.now();
      if (pending.current !== null) change.current(pending.current);
    }, wait);
  }, []);
}

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
  const send = useSend(onChange);

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

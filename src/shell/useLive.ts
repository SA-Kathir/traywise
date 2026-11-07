import { useEffect, useState } from "react";

type LiveEvent<T> = { listen: (handler: (e: { payload: T }) => void) => Promise<() => void> };

// Tiles still waiting for their first data (or for their code to load): the panel holds its entrance
// until there are none, so it never comes in half built.
let pending = 0;

/** A Suspense fallback that counts as "still loading" while it shows. */
export function Pending() {
  useEffect(() => {
    pending++;
    return () => {
      pending--;
    };
  }, []);
  return null;
}

/** Resolves once nothing is loading, for two frames in a row (a tile's code arriving starts its data loading). */
export function settled(): Promise<void> {
  return new Promise((resolve) => {
    let calm = 0;
    const check = () => {
      calm = pending === 0 ? calm + 1 : 0;
      if (calm >= 2) resolve();
      else requestAnimationFrame(check);
    };
    requestAnimationFrame(check);
  });
}

// The last value each module sent, so a tile that is mounted again (coming back from Settings or another tab)
// shows it at once instead of empty until its data arrives again. Keyed by the fetch command.
const last = new Map<unknown, unknown>();

/** A module's state from Rust: fetched once, then kept current by its change event. Null until the first value
 *  (or, when remounted, the last one it had). */
export function useLive<T>(fetch: () => Promise<T>, changed: LiveEvent<T>): T | null {
  const [value, setValue] = useState<T | null>(() => (last.get(fetch) as T | undefined) ?? null);
  const keep = (v: T) => {
    last.set(fetch, v);
    setValue(v);
  };
  useEffect(() => {
    let live = false; // an event beat the first fetch: don't overwrite it with older data
    let waiting = !last.has(fetch); // a remount with a value to show isn't waiting for anything
    const arrived = () => {
      if (waiting) pending--;
      waiting = false;
    };
    if (waiting) pending++;
    const unlisten = changed.listen((e) => {
      live = true;
      keep(e.payload);
      arrived();
    });
    fetch().then(
      (v) => {
        live || keep(v);
        arrived();
      },
      arrived,
    );
    return () => {
      arrived();
      unlisten.then((stop) => stop());
    };
    // Once per mount: callers pass fixed commands and events from bindings.ts.
  }, []);
  return value;
}

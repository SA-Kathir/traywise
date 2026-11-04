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

/** A module's state from Rust: fetched once, then kept current by its change event. Null until the first value. */
export function useLive<T>(fetch: () => Promise<T>, changed: LiveEvent<T>): T | null {
  const [value, setValue] = useState<T | null>(null);
  useEffect(() => {
    let live = false; // an event beat the first fetch: don't overwrite it with older data
    let waiting = true;
    const arrived = () => {
      if (waiting) pending--;
      waiting = false;
    };
    pending++;
    const unlisten = changed.listen((e) => {
      live = true;
      setValue(e.payload);
      arrived();
    });
    fetch().then(
      (v) => {
        live || setValue(v);
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

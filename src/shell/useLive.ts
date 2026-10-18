import { useEffect, useState } from "react";

type LiveEvent<T> = { listen: (handler: (e: { payload: T }) => void) => Promise<() => void> };

/** A module's state from Rust: fetched once, then kept current by its change event. Null until the first value. */
export function useLive<T>(fetch: () => Promise<T>, changed: LiveEvent<T>): T | null {
  const [value, setValue] = useState<T | null>(null);
  useEffect(() => {
    let live = false; // an event beat the first fetch: don't overwrite it with older data
    const unlisten = changed.listen((e) => {
      live = true;
      setValue(e.payload);
    });
    fetch().then((v) => live || setValue(v));
    return () => {
      unlisten.then((stop) => stop());
    };
    // Once per mount: callers pass fixed commands and events from bindings.ts.
  }, []);
  return value;
}

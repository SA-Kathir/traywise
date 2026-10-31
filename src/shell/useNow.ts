import { useEffect, useState } from "react";

/** The current time, ticking every second while `ticking`. */
export function useNow(ticking: boolean) {
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    if (!ticking) return;
    setNow(Date.now());
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, [ticking]);
  return now;
}

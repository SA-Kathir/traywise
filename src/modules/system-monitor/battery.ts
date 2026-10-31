import type { Battery } from "../../bindings";

/** "79%, charging", "79%, 3 h 5 min left" or "79%". Shared with the Power module. */
export function battery({ percent, charging, secondsLeft }: Battery) {
  if (charging) return `${percent}%, charging`;
  if (secondsLeft === null) return `${percent}%`;
  const minutes = Math.round(secondsLeft / 60);
  const h = Math.floor(minutes / 60);
  return `${percent}%, ${h > 0 ? `${h} h ${minutes % 60} min` : `${minutes} min`} left`;
}

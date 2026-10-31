import { lazy } from "react";
import type { ModuleUi } from "..";

const clock: ModuleUi = {
  name: "Clock",
  icon: String.fromCodePoint(0xe823), // Segoe Fluent "Recent" (a clock face)
  category: "tools",
  description: "Time, date, a 2nd zone",
  idle: "does nothing",
  tiles: { clock: { name: "Clock", sizes: ["1x1", "2x1", "4x1", "2x2", "4x2"] } },
  Tiles: lazy(() => import("./Panel").then((m) => ({ default: m.Tiles }))),
  Panel: lazy(() => import("./Panel")),
};

export default clock;

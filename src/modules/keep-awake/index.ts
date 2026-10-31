import { lazy } from "react";
import type { ModuleUi } from "..";

const keepAwake: ModuleUi = {
  name: "Keep awake",
  icon: String.fromCodePoint(0xec32), // Segoe Fluent "Cafe" (a cup)
  category: "system",
  description: "Stop the PC sleeping",
  idle: "waits for power",
  tiles: { "keep-awake": { name: "Keep awake", sizes: ["1x1", "2x1", "2x2"] } },
  Tiles: lazy(() => import("./Panel").then((m) => ({ default: m.Tiles }))),
  Panel: lazy(() => import("./Panel")),
};

export default keepAwake;

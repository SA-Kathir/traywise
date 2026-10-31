import { lazy } from "react";
import type { ModuleUi } from "..";

const power: ModuleUi = {
  name: "Power",
  icon: String.fromCodePoint(0xec49), // Segoe Fluent "Speed Medium"
  category: "system",
  description: "Power mode, battery",
  idle: "waits for power",
  tiles: {
    mode: { name: "Power mode", sizes: ["1x1", "2x1"] },
    battery: { name: "Battery", sizes: ["1x1", "2x1"] },
  },
  Tiles: lazy(() => import("./Panel").then((m) => ({ default: m.Tiles }))),
  Panel: lazy(() => import("./Panel")),
};

export default power;

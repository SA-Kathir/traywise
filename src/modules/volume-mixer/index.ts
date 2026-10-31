import { lazy } from "react";
import type { ModuleUi } from "..";

const volumeMixer: ModuleUi = {
  name: "Volume mixer",
  icon: String.fromCodePoint(0xe767), // Segoe Fluent "Volume"
  category: "sound",
  description: "Volume for every app",
  idle: "waits for volume",
  tiles: { mixer: { name: "Volume mixer", sizes: ["1x1", "2x1", "4x1", "2x2", "4x2"] } },
  Tiles: lazy(() => import("./Panel").then((m) => ({ default: m.Tiles }))),
  Panel: lazy(() => import("./Panel")),
};

export default volumeMixer;

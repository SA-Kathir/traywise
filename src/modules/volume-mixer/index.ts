import { lazy } from "react";
import type { ModuleUi } from "..";

const volumeMixer: ModuleUi = {
  name: "Volume mixer",
  icon: String.fromCodePoint(0xe767), // Segoe Fluent "Volume"
  category: "sound",
  description: "Master volume, plus volume and mute for each app.",
  idle: "Listens for volume changes",
  tiles: { mixer: { name: "Volume mixer", sizes: ["1x1", "2x1", "4x1", "2x2", "4x2"] } },
  Tiles: lazy(() => import("./Panel").then((m) => ({ default: m.Tiles }))),
  Panel: lazy(() => import("./Panel")),
};

export default volumeMixer;

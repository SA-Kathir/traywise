import { lazy } from "react";
import type { ModuleUi } from "..";

const displays: ModuleUi = {
  name: "Displays",
  icon: String.fromCodePoint(0xe706), // Segoe Fluent "Brightness"
  category: "system",
  description: "Brightness, Project",
  idle: "off till opened",
  tiles: {
    brightness: { name: "Brightness", sizes: ["1x1", "2x1", "4x1", "2x2", "4x2"] },
    project: { name: "Project", sizes: ["1x1", "2x1"] },
  },
  Tiles: lazy(() => import("./Panel").then((m) => ({ default: m.Tiles }))),
  Panel: lazy(() => import("./Panel")),
};

export default displays;

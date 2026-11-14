import { lazy } from "react";
import type { ModuleUi } from "..";

const displays: ModuleUi = {
  name: "Displays",
  icon: String.fromCodePoint(0xe706), // Segoe Fluent "Brightness"
  category: "system",
  description: "Brightness, refresh rate, Project",
  idle: "off till opened",
  tiles: {
    brightness: {
      name: "Brightness",
      sizes: ["1x1", "2x1", "4x1", "2x2", "4x2"],
      notes: { "1x1": "a ring; scroll to change", "2x1": "a bar", "4x1": "a bar and the screen", "2x2": "a dial for one screen", "4x2": "a bar for each screen" },
    },
    "brightness-sliders": { name: "Brightness", group: "brightness", sizes: ["2x2"], notes: { "2x2": "a slider for each screen" } },
    refresh: { name: "Refresh rate", sizes: ["1x1", "2x1"] },
    project: { name: "Project", sizes: ["1x1", "2x1", "2x2", "4x1", "4x2"] },
  },
  Tiles: lazy(() => import("./Panel").then((m) => ({ default: m.Tiles }))),
  Panel: lazy(() => import("./Panel")),
};

export default displays;

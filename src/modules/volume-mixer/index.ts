import { lazy } from "react";
import type { ModuleUi } from "..";

const volumeMixer: ModuleUi = {
  name: "Volume mixer",
  icon: String.fromCodePoint(0xe767), // Segoe Fluent "Volume"
  category: "sound",
  description: "Volume for every app",
  idle: "waits for volume",
  tiles: {
    mixer: {
      name: "Volume mixer",
      sizes: ["1x1", "2x1", "4x1", "2x2", "4x2"],
      notes: { "1x1": "a ring; scroll or tap to change", "2x1": "a bar", "4x1": "a bar and the device", "2x2": "a dial for the main volume", "4x2": "the main volume and each app" },
    },
    "mixer-sliders": { name: "Volume mixer", group: "mixer", sizes: ["2x2"], notes: { "2x2": "a slider each for the main volume and your apps" } },
  },
  Tiles: lazy(() => import("./Panel").then((m) => ({ default: m.Tiles }))),
  Panel: lazy(() => import("./Panel")),
};

export default volumeMixer;

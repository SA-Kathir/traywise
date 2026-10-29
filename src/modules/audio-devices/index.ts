import { lazy } from "react";
import type { ModuleUi } from "..";

const audioDevices: ModuleUi = {
  name: "Audio devices",
  icon: String.fromCodePoint(0xe7f5), // Segoe Fluent "Speakers"
  category: "sound",
  description: "Switch outputs, and mute every mic with a shortcut.",
  idle: "Listens for device changes",
  tiles: {
    output: { name: "Output", sizes: ["1x1", "2x1", "2x2"] },
    mics: { name: "Microphones", sizes: ["1x1", "2x1"] },
  },
  Tiles: lazy(() => import("./Panel").then((m) => ({ default: m.Tiles }))),
  Panel: lazy(() => import("./Panel")),
};

export default audioDevices;

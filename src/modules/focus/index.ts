import { lazy } from "react";
import type { ModuleUi } from "..";

const focus: ModuleUi = {
  name: "Focus",
  icon: String.fromCodePoint(0xe1d2), // Segoe Fluent "Target"
  category: "tools",
  description: "Pomodoro and Do not disturb",
  idle: "waits for changes",
  tiles: {
    focus: { name: "Focus", sizes: ["1x1", "2x1", "2x2", "4x2"] },
    dnd: { name: "Do not disturb", sizes: ["1x1", "2x1"] },
  },
  Tiles: lazy(() => import("./Tiles")),
  Panel: lazy(() => import("./Panel")),
};

export default focus;

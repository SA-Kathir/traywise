import { lazy } from "react";
import type { ModuleUi } from "..";

const quickToggles: ModuleUi = {
  name: "Quick toggles",
  icon: String.fromCodePoint(0xe771), // Segoe Fluent "Personalize"
  category: "tools",
  description: "Dark mode, icons, lock",
  idle: "does nothing",
  tiles: {
    dark: { name: "Dark mode", sizes: ["1x1", "2x1"] },
    icons: { name: "Desktop icons", sizes: ["1x1", "2x1"] },
    lock: { name: "Lock", sizes: ["1x1", "2x1"] },
  },
  Tiles: lazy(() => import("./Tiles")),
};

export default quickToggles;

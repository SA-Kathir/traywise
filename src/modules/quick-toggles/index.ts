import { lazy } from "react";
import type { ModuleUi } from "..";

const quickToggles: ModuleUi = {
  name: "Quick toggles",
  icon: String.fromCodePoint(0xe771), // Segoe Fluent "Personalize"
  category: "tools",
  description: "Wi-Fi, Bluetooth, dark mode, desktop icons and lock, one click each.",
  idle: "Nothing at rest",
  tiles: {
    wifi: { name: "Wi-Fi", sizes: ["1x1", "2x1"] },
    bluetooth: { name: "Bluetooth", sizes: ["1x1", "2x1"] },
    dark: { name: "Dark mode", sizes: ["1x1", "2x1"] },
    icons: { name: "Desktop icons", sizes: ["1x1", "2x1"] },
    lock: { name: "Lock", sizes: ["1x1", "2x1"] },
  },
  Tiles: lazy(() => import("./Tiles")),
};

export default quickToggles;

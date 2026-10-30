import { lazy } from "react";
import type { ModuleUi } from "..";

const wifi: ModuleUi = {
  name: "Wi-Fi",
  icon: String.fromCodePoint(0xe701), // Segoe Fluent "Wifi"
  category: "connect",
  description: "Switch Wi-Fi, see the network you're on, and join another.",
  idle: "Nothing at rest",
  tiles: { wifi: { name: "Wi-Fi", sizes: ["1x1", "2x1", "2x2"] } },
  Tiles: lazy(() => import("./Panel").then((m) => ({ default: m.Tiles }))),
  Panel: lazy(() => import("./Panel")),
};

export default wifi;

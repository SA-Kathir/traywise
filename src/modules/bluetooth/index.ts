import { lazy } from "react";
import type { ModuleUi } from "..";

const bluetooth: ModuleUi = {
  name: "Bluetooth",
  icon: String.fromCodePoint(0xe702), // Segoe Fluent "Bluetooth"
  category: "connect",
  description: "Connect audio devices",
  idle: "does nothing",
  tiles: { bluetooth: { name: "Bluetooth", sizes: ["1x1", "2x1", "2x2"] } },
  Tiles: lazy(() => import("./Panel").then((m) => ({ default: m.Tiles }))),
  Panel: lazy(() => import("./Panel")),
};

export default bluetooth;

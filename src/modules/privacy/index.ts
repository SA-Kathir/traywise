import { lazy } from "react";
import type { ModuleUi } from "..";

const privacy: ModuleUi = {
  name: "Privacy",
  icon: String.fromCodePoint(0xe72e), // Segoe Fluent "Lock"
  category: "system",
  description: "Mic and camera in use",
  idle: "waits for changes",
  tiles: {
    indicator: { name: "Mic and camera", sizes: ["1x1", "2x1"] },
  },
  Tiles: lazy(() => import("./Tiles")),
};

export default privacy;

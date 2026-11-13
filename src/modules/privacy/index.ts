import { lazy } from "react";
import type { ModuleUi } from "..";

const privacy: ModuleUi = {
  name: "Privacy",
  icon: String.fromCodePoint(0xe72e), // Segoe Fluent "Lock"
  category: "system",
  description: "Mic and camera in use",
  idle: "off till opened",
  tiles: {
    indicator: {
      name: "Mic and camera",
      sizes: ["1x1", "2x1"],
      notes: { "1x1": "lit while an app uses the mic or camera", "2x1": "and which apps" },
    },
  },
  Tiles: lazy(() => import("./Tiles")),
};

export default privacy;

import { lazy } from "react";
import type { ModuleUi } from "..";

const quickToggles: ModuleUi = {
  name: "Quick toggles",
  icon: String.fromCodePoint(0xe771), // Segoe Fluent "Personalize"
  Summary: lazy(() => import("./Tiles")),
};

export default quickToggles;

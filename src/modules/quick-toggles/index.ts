import { lazy } from "react";
import type { ModuleUi } from "..";

const quickToggles: ModuleUi = {
  name: "Quick toggles",
  icon: String.fromCodePoint(0xe771), // Segoe Fluent "Personalize"
  category: "tools",
  description: "Dark mode, desktop icons and lock, one click each.",
  idle: "Nothing at rest",
  Summary: lazy(() => import("./Tiles")),
};

export default quickToggles;

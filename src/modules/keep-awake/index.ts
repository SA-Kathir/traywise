import { lazy } from "react";
import type { ModuleUi } from "..";

const keepAwake: ModuleUi = {
  name: "Keep awake",
  icon: String.fromCodePoint(0xe706), // Segoe Fluent "Brightness" (a sun)
  Summary: lazy(() => import("./Panel").then((m) => ({ default: m.Summary }))),
  Panel: lazy(() => import("./Panel")),
};

export default keepAwake;

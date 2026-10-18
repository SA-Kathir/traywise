import { lazy } from "react";
import type { ModuleUi } from "..";

const volumeMixer: ModuleUi = {
  name: "Volume mixer",
  icon: String.fromCodePoint(0xe767), // Segoe Fluent "Volume"
  Summary: lazy(() => import("./Panel").then((m) => ({ default: m.Summary }))),
  Panel: lazy(() => import("./Panel")),
};

export default volumeMixer;

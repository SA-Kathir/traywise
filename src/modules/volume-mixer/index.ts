import { lazy } from "react";
import type { ModuleUi } from "..";

const volumeMixer: ModuleUi = {
  name: "Volume mixer",
  icon: String.fromCodePoint(0xe767), // Segoe Fluent "Volume"
  category: "sound",
  description: "Master volume, plus volume and mute for each app.",
  idle: "Listens for volume changes",
  Summary: lazy(() => import("./Panel").then((m) => ({ default: m.Summary }))),
  Panel: lazy(() => import("./Panel")),
};

export default volumeMixer;

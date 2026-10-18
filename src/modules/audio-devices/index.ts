import { lazy } from "react";
import type { ModuleUi } from "..";

const audioDevices: ModuleUi = {
  name: "Audio devices",
  icon: String.fromCodePoint(0xe7f5), // Segoe Fluent "Speakers"
  Panel: lazy(() => import("./Panel")),
};

export default audioDevices;

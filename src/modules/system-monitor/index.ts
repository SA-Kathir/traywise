import { lazy } from "react";
import type { ModuleUi } from "..";

const systemMonitor: ModuleUi = {
  name: "System monitor",
  icon: String.fromCodePoint(0xe9d9), // Segoe Fluent "Diagnostic"
  Summary: lazy(() => import("./Panel").then((m) => ({ default: m.Summary }))),
  Panel: lazy(() => import("./Panel")),
};

export default systemMonitor;

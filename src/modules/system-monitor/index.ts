import { lazy } from "react";
import type { ModuleUi } from "..";

const systemMonitor: ModuleUi = {
  name: "System monitor",
  icon: String.fromCodePoint(0xe9d9), // Segoe Fluent "Diagnostic"
  category: "system",
  description: "CPU, memory, network, battery and disk.",
  idle: "Only while the panel is open",
  Summary: lazy(() => import("./Panel").then((m) => ({ default: m.Summary }))),
  Panel: lazy(() => import("./Panel")),
};

export default systemMonitor;

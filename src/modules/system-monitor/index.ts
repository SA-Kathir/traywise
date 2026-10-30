import { lazy } from "react";
import type { ModuleUi } from "..";

const systemMonitor: ModuleUi = {
  name: "System monitor",
  icon: String.fromCodePoint(0xe9d9), // Segoe Fluent "Diagnostic"
  category: "system",
  description: "CPU, memory, network, battery and disk.",
  idle: "Only while the panel is open",
  tiles: {
    cpu: { name: "CPU", sizes: ["1x1", "2x1", "2x2"] },
    memory: { name: "Memory", sizes: ["1x1", "2x1", "2x2"] },
    monitor: { name: "Two readings", sizes: ["4x1", "4x2"] },
  },
  Tiles: lazy(() => import("./Panel").then((m) => ({ default: m.Tiles }))),
  Panel: lazy(() => import("./Panel")),
};

export default systemMonitor;

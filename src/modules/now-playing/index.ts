import { lazy } from "react";
import type { ModuleUi } from "..";

const nowPlaying: ModuleUi = {
  name: "Now playing",
  icon: String.fromCodePoint(0xe8d6), // Segoe Fluent "Audio" (a music note)
  category: "sound",
  description: "Control what's playing",
  idle: "waits for media",
  tiles: { media: { name: "Now playing", sizes: ["1x1", "2x1", "2x2", "4x2"] } },
  Tiles: lazy(() => import("./Panel").then((m) => ({ default: m.Tiles }))),
  Panel: lazy(() => import("./Panel")),
};

export default nowPlaying;

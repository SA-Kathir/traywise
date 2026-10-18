import type { ComponentType, LazyExoticComponent } from "react";
import type { ModuleId } from "../bindings";
import audioDevices from "./audio-devices";
import keepAwake from "./keep-awake";
import systemMonitor from "./system-monitor";
import volumeMixer from "./volume-mixer";

/** What the shell needs to show a module. Both parts are lazy: a disabled module's code never loads. */
export type ModuleUi = {
  name: string;
  /** A Segoe Fluent Icons glyph. */
  icon: string;
  /** Its piece of the home view: tiles (`Tile`) or one full-width row (class `wide`). */
  Summary: LazyExoticComponent<ComponentType<{ open: () => void }>>;
  /** Its own page, opened from the summary. */
  Panel: LazyExoticComponent<ComponentType>;
};

/** Every module's UI, keyed by the ids Rust defines. TypeScript errors if one is missing.
 *  This order is the home view's and Settings' order: tiles first, then the full-width rows. */
export const modules: Record<ModuleId, ModuleUi> = {
  "keep-awake": keepAwake,
  "audio-devices": audioDevices,
  "volume-mixer": volumeMixer,
  "system-monitor": systemMonitor,
};

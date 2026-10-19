import type { ComponentType, LazyExoticComponent } from "react";
import type { ModuleId } from "../bindings";
import audioDevices from "./audio-devices";
import keepAwake from "./keep-awake";
import quickToggles from "./quick-toggles";
import systemMonitor from "./system-monitor";
import volumeMixer from "./volume-mixer";

/** The panel's tabs after Home. A tab shows only when one of its modules is on and has a page. */
export const categories = {
  sound: { name: "Sound", icon: String.fromCodePoint(0xe767) }, // Segoe Fluent "Volume"
  system: { name: "System", icon: String.fromCodePoint(0xe770) }, // Segoe Fluent "System"
  tools: { name: "Tools", icon: String.fromCodePoint(0xe90f) }, // Segoe Fluent "Repair"
};
export type Category = keyof typeof categories;

/** What the shell needs to show a module. Both parts are lazy: a disabled module's code never loads. */
export type ModuleUi = {
  name: string;
  /** A Segoe Fluent Icons glyph. */
  icon: string;
  category: Category;
  /** One line on the Modules page: what it does. */
  description: string;
  /** What it costs while nothing is happening, on the Modules page. */
  idle: string;
  /** Its piece of the home view: tiles (`Tile`) or one full-width row (class `wide`). */
  Summary: LazyExoticComponent<ComponentType<{ open: () => void }>>;
  /** Its own page, opened from the summary. Quick toggles has none. */
  Panel?: LazyExoticComponent<ComponentType>;
};

/** Every module's UI, keyed by the ids Rust defines. TypeScript errors if one is missing.
 *  This order is the home view's, the tabs' and the Modules page's order: tiles first, then the
 *  full-width rows. */
export const modules: Record<ModuleId, ModuleUi> = {
  "keep-awake": keepAwake,
  "audio-devices": audioDevices,
  "quick-toggles": quickToggles,
  "volume-mixer": volumeMixer,
  "system-monitor": systemMonitor,
};

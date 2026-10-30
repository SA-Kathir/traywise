import type { ComponentType, LazyExoticComponent } from "react";
import type { ModuleId } from "../bindings";
import audioDevices from "./audio-devices";
import keepAwake from "./keep-awake";
import nowPlaying from "./now-playing";
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

/** A tile's size on the home board, in grid cells: columns × rows. The board is 4 columns wide. */
export type Size = "1x1" | "2x1" | "4x1" | "2x2" | "4x2";

/** What a module's `Tiles` component is given: which of its tiles to draw, and at what size. */
export type TileProps = { kind: string; size: Size; open: () => void };

/** What the shell needs to show a module. The components are lazy: a disabled module's code never loads. */
export type ModuleUi = {
  name: string;
  /** A Segoe Fluent Icons glyph. */
  icon: string;
  category: Category;
  /** One line on the Modules page: what it does. */
  description: string;
  /** What it costs while nothing is happening, on the Modules page. */
  idle: string;
  /** The tiles it offers for the home board, by kind: a name, and the sizes it comes in, smallest
   *  first (resizing cycles through them). Plain data, so listing them doesn't load the module. */
  tiles: Record<string, { name: string; sizes: Size[] }>;
  /** Draws one of its tiles. */
  Tiles: LazyExoticComponent<ComponentType<TileProps>>;
  /** Its own page, opened from its tiles. Quick toggles has none. */
  Panel?: LazyExoticComponent<ComponentType>;
};

/** Every module's UI, keyed by the ids Rust defines. TypeScript errors if one is missing.
 *  This order is the tabs' and the Modules page's order. */
export const modules: Record<ModuleId, ModuleUi> = {
  "keep-awake": keepAwake,
  "audio-devices": audioDevices,
  "quick-toggles": quickToggles,
  "volume-mixer": volumeMixer,
  "now-playing": nowPlaying,
  "system-monitor": systemMonitor,
};

/** A tile on the board is "<module id>/<kind>", like "quick-toggles/dark": this is its module. */
export const tileModule = (id: string) => id.slice(0, id.indexOf("/")) as ModuleId;

/** A new user's board, in order; tiles of modules that are off are left out. Packed, it's:
 *  Wi-Fi, Bluetooth and the mixer (2×2) on top, Dark mode and Microphones under the first two, then
 *  Keep awake and Output side by side, the system monitor across, and Now playing at the bottom.
 *  (Lock and Desktop icons are in Add tiles: with them the rows don't come out even.)
 *  Switching a module on later adds its tiles from here. */
export const defaultBoard: [id: string, size: Size][] = [
  ["quick-toggles/wifi", "1x1"],
  ["quick-toggles/bluetooth", "1x1"],
  ["volume-mixer/mixer", "2x2"],
  ["quick-toggles/dark", "1x1"],
  ["audio-devices/mics", "1x1"],
  ["keep-awake/keep-awake", "2x1"],
  ["audio-devices/output", "2x1"],
  ["system-monitor/monitor", "4x2"],
  ["now-playing/media", "4x2"],
];

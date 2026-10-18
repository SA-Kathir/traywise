import type { ComponentType, LazyExoticComponent } from "react";
import type { ModuleId } from "../bindings";
import keepAwake from "./keep-awake";
import volumeMixer from "./volume-mixer";

/** What the shell needs to show a module. The panel is lazy: a disabled module's code never loads. */
export type ModuleUi = {
  name: string;
  /** A Segoe Fluent Icons glyph. */
  icon: string;
  Panel: LazyExoticComponent<ComponentType>;
};

/** Every module's UI, keyed by the ids Rust defines. TypeScript errors if one is missing. */
export const modules: Record<ModuleId, ModuleUi> = {
  "volume-mixer": volumeMixer,
  "keep-awake": keepAwake,
};

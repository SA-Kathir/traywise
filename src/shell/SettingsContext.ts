import { createContext, useContext } from "react";
import type { Settings } from "../bindings";

/** The settings and how to save them, for module tiles and pages with options of their own. Saving
 *  through here (not commands.setSettings) keeps the app's copy current, so a later save, like a
 *  tile moved on the board, doesn't undo it. */
export const SettingsContext = createContext<{
  settings: Settings;
  save: (next: Settings) => void;
  /** Leaves a module's page for Home, on the page that has this tile (like "focus/focus"). Does nothing if no page has it. */
  showTile: (tileId: string) => void;
} | null>(null);

export const useSettings = () => useContext(SettingsContext)!;

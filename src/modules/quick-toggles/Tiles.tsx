import { commands, events } from "../../bindings";
import { Tile } from "../../shell/Tile";
import { useLive } from "../../shell/useLive";
import type { TileProps } from "..";

// Segoe Fluent Icons glyphs.
// Each tile's icon shows its state: a moon or a sun, the desktop or a crossed-out eye.
const MOON = String.fromCodePoint(0xe708);
const SUN = String.fromCodePoint(0xe706);
const DESKTOP = String.fromCodePoint(0xe7f4);
const HIDDEN = String.fromCodePoint(0xed1a);
const LOCK = String.fromCodePoint(0xe72e);

/** Home tiles: dark mode, desktop icons, lock. They have no page of their own. */
export default function Tiles({ kind, size }: TileProps) {
  const state = useLive(commands.quickTogglesState, events.quickTogglesChanged);
  if (!state) return null;
  if (kind === "dark")
    return (
      <Tile
        size={size}
        icon={state.dark ? MOON : SUN}
        label="Dark mode"
        detail={state.dark ? "On" : "Off"}
        on={state.dark}
        onToggle={() => commands.quickTogglesSetDark(!state.dark)}
      />
    );
  if (kind === "icons")
    return (
      <Tile
        size={size}
        icon={state.desktopIcons ? DESKTOP : HIDDEN}
        label="Desktop icons"
        detail={state.desktopIcons ? "Shown" : "Hidden"}
        on={state.desktopIcons}
        onToggle={() => commands.quickTogglesSetDesktopIcons(!state.desktopIcons)}
      />
    );
  return (
    <Tile
      size={size}
      icon={LOCK}
      label="Lock"
      detail={size === "1x1" ? "Lock" : "Lock this PC"}
      onToggle={() => commands.quickTogglesLock()}
    />
  );
}

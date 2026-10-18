import { commands, events } from "../../bindings";
import { Tile } from "../../shell/Tile";
import { useLive } from "../../shell/useLive";

// Segoe Fluent Icons glyphs.
const MOON = String.fromCodePoint(0xe708);
const DESKTOP = String.fromCodePoint(0xe7f4);
const LOCK = String.fromCodePoint(0xe72e);

/** Home tiles: dark mode, desktop icons, lock. They have no page of their own. */
export default function Tiles() {
  const state = useLive(commands.quickTogglesState, events.quickTogglesChanged);
  if (!state) return null;
  return (
    <>
      <Tile
        icon={MOON}
        label="Dark mode"
        detail={state.dark ? "On" : "Off"}
        on={state.dark}
        onToggle={() => commands.quickTogglesSetDark(!state.dark)}
      />
      <Tile
        icon={DESKTOP}
        label="Desktop icons"
        detail={state.desktopIcons ? "Shown" : "Hidden"}
        on={state.desktopIcons}
        onToggle={() => commands.quickTogglesSetDesktopIcons(!state.desktopIcons)}
      />
      <Tile icon={LOCK} label="Lock" detail="Lock this PC" onToggle={() => commands.quickTogglesLock()} />
    </>
  );
}

import { commands, events } from "../../bindings";
import { Tile } from "../../shell/Tile";
import { useLive } from "../../shell/useLive";
import type { TileProps } from "..";

// Segoe Fluent Icons glyphs.
const MOON = String.fromCodePoint(0xe708);
const DESKTOP = String.fromCodePoint(0xe7f4);
const LOCK = String.fromCodePoint(0xe72e);
const WIFI = String.fromCodePoint(0xe701);
const BLUETOOTH = String.fromCodePoint(0xe702);

/** Home tiles: Wi-Fi, Bluetooth, dark mode, desktop icons, lock. They have no page of their own. */
export default function Tiles({ kind, size }: TileProps) {
  const state = useLive(commands.quickTogglesState, events.quickTogglesChanged);
  if (!state) return null;
  if (kind === "wifi" || kind === "bluetooth") {
    const on = state[kind]; // null: this PC has no such radio
    return (
      <Tile
        size={size}
        icon={kind === "wifi" ? WIFI : BLUETOOTH}
        label={kind === "wifi" ? "Wi-Fi" : "Bluetooth"}
        detail={on === null ? "Not available" : on ? "On" : "Off"}
        on={on === true}
        onToggle={on === null ? undefined : () => commands.quickTogglesSetRadio(kind, !on)}
      />
    );
  }
  if (kind === "dark")
    return (
      <Tile
        size={size}
        icon={MOON}
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
        icon={DESKTOP}
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

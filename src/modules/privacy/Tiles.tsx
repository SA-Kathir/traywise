import { commands, events } from "../../bindings";
import { Tile } from "../../shell/Tile";
import { useLive } from "../../shell/useLive";
import type { TileProps } from "..";

// Segoe Fluent Icons glyphs.
const MIC = String.fromCodePoint(0xe720);
const CAMERA = String.fromCodePoint(0xe722);
const SHIELD = String.fromCodePoint(0xea18);

/** "Zoom", "Zoom and Teams", "Zoom, Teams and 2 more". */
function names(apps: string[]) {
  if (apps.length <= 2) return apps.join(" and ");
  return `${apps[0]}, ${apps[1]} and ${apps.length - 2} more`;
}

/** Home tile: lit while an app is using the microphone or camera right now, and which. Read when the
 *  panel opens (Windows has no notification for it), so it shows what was true at that moment. */
export default function Tiles({ size }: TileProps) {
  const state = useLive(commands.privacyState, events.privacyChanged);
  if (!state) return null;
  const { microphone, camera } = state;
  const on = microphone.length > 0 || camera.length > 0;
  const icon = camera.length > 0 && microphone.length === 0 ? CAMERA : on ? MIC : SHIELD;
  const using = [microphone.length > 0 && "mic", camera.length > 0 && "camera"].filter(Boolean).join(" + ");
  const who = names([...new Set([...microphone, ...camera])]);
  return (
    <Tile
      size={size}
      icon={icon}
      label={on ? "In use" : "Privacy"}
      detail={on ? (size === "1x1" ? using : `${using}: ${who}`) : size === "1x1" ? "Idle" : "Mic and camera idle"}
      on={on}
    />
  );
}

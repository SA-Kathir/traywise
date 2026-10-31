import { commands, events, type Battery, type PowerMode } from "../../bindings";
import { Tile, TileCard } from "../../shell/Tile";
import { useLive } from "../../shell/useLive";
import { battery as batteryText } from "../system-monitor/battery";
import type { TileProps } from "..";

// Segoe Fluent Icons glyphs.
const MODE_ICONS: Record<PowerMode, string> = {
  efficiency: String.fromCodePoint(0xec48), // "Speed Off"
  balanced: String.fromCodePoint(0xec49), // "Speed Medium"
  performance: String.fromCodePoint(0xec4a), // "Speed High"
};
const CHARGING = String.fromCodePoint(0xe83e);

const MODES: { mode: PowerMode; label: string }[] = [
  { mode: "efficiency", label: "Efficiency" },
  { mode: "balanced", label: "Balanced" },
  { mode: "performance", label: "Performance" },
];

/** The next mode a tap on the 1×1 tile picks, round and round. */
const nextMode = (mode: PowerMode) => MODES[(MODES.findIndex((m) => m.mode === mode) + 1) % MODES.length].mode;

/** Segoe Fluent's battery glyphs: Battery0 to Battery9 are U+E850 to U+E859; Battery10 is U+E83F. */
const batteryIcon = ({ percent, charging }: Battery) =>
  charging ? CHARGING : String.fromCodePoint(percent >= 95 ? 0xe83f : 0xe850 + Math.floor(percent / 10));

const usePower = () => useLive(commands.powerState, events.powerChanged);

/** Home tiles: the power mode (a tap cycles it; at 2×1, all three) and the battery. */
export function Tiles({ kind, size, open }: TileProps) {
  const state = usePower();
  if (!state) return null;
  if (kind === "battery") {
    const b = state.battery;
    return (
      <Tile
        size={size}
        icon={b ? batteryIcon(b) : String.fromCodePoint(0xe83f)}
        label="Battery"
        detail={!b ? "No battery" : size === "1x1" ? `${b.percent}%` : batteryText(b)}
        onOpen={open}
      />
    );
  }
  const mode = state.mode;
  if (size === "2x1" && mode)
    return (
      <TileCard size={size} label="Power mode" onOpen={open}>
        <Modes mode={mode} />
      </TileCard>
    );
  return (
    <Tile
      size={size}
      icon={MODE_ICONS[mode ?? "balanced"]}
      label="Power mode"
      detail={mode ? MODES.find((m) => m.mode === mode)!.label : "Unavailable"}
      onToggle={mode ? () => commands.powerSetMode(nextMode(mode)) : undefined}
      onOpen={open}
    />
  );
}

function Modes({ mode }: { mode: PowerMode }) {
  return (
    <div className="segmented" role="radiogroup" aria-label="Power mode">
      {MODES.map((m) => (
        <button key={m.mode} role="radio" aria-checked={mode === m.mode} className={mode === m.mode ? "on" : ""} onClick={() => commands.powerSetMode(m.mode)}>
          {m.label}
        </button>
      ))}
    </div>
  );
}

export default function Panel() {
  const state = usePower();
  if (!state) return null;
  return (
    <>
      <h2>Power mode</h2>
      {state.mode ? (
        <Modes mode={state.mode} />
      ) : (
        <p className="hint">Windows offers power modes only on the Balanced power plan.</p>
      )}
      {state.battery && (
        <>
          <h2>Battery</h2>
          <p className="row">{batteryText(state.battery)}</p>
        </>
      )}
      <div className="row">
        <span className="grow">Energy saver: {state.energySaver ? "on" : "off"}</span>
        <button onClick={() => commands.openSettings("energySaver")}>Open in Settings</button>
      </div>
      <p className="hint">Windows doesn't let apps switch Energy saver, so it's changed in Settings.</p>
    </>
  );
}

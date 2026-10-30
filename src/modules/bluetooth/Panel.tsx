import { useState } from "react";
import { commands, events } from "../../bindings";
import { Tile } from "../../shell/Tile";
import { useLive } from "../../shell/useLive";
import type { TileProps } from "..";

// Segoe Fluent Icons glyphs. Windows has no "Bluetooth off" one, so that one's drawn: the rune, crossed.
const BLUETOOTH = String.fromCodePoint(0xe702);
const BLUETOOTH_OFF = (
  <svg className="media-glyph" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
    <path d="M4.5 5l7 6-3.5 3V2l3.5 3-7 6" />
    <path d="M2.5 2.5l11 11" />
  </svg>
);
const HEADPHONES = String.fromCodePoint(0xe7f6);
const CHEVRON = String.fromCodePoint(0xe76c);

const useBluetooth = () => useLive(commands.bluetoothState, events.bluetoothChanged);

/** Home tile: Bluetooth on or off, and what's connected. */
export function Tiles({ size, open }: TileProps) {
  const s = useBluetooth();
  if (!s) return null;
  const connected = s.devices.filter((d) => d.connected);
  // 1×1: on or off ("Bluetooth" doesn't fit in the Dot look's OCR-A).
  const detail =
    s.on === null
      ? "Not available"
      : !s.on
        ? "Off"
        : connected.length > 0
          ? size === "1x1"
            ? "On"
            : connected[0].name + (connected.length > 1 ? ` +${connected.length - 1}` : "")
          : size === "1x1"
            ? "On"
            : "Nothing connected";
  return (
    <Tile
      size={size}
      icon={s.on ? BLUETOOTH : BLUETOOTH_OFF}
      label="Bluetooth"
      detail={detail}
      on={s.on === true}
      onToggle={s.on === null ? undefined : () => commands.bluetoothSetOn(!s.on)}
      onOpen={open}
    />
  );
}

/** The page: the switch and the paired devices; headphones and speakers connect from here. */
export default function Panel() {
  const s = useBluetooth();
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  if (!s) return null;
  if (s.on === null) return <p className="hint">This PC has no Bluetooth.</p>;

  async function connect(id: string, name: string, on: boolean) {
    setBusy(id);
    setError(null);
    const r = await commands.bluetoothConnect(id, on);
    setBusy(null);
    if (r.status === "error") setError(`${name}: ${r.error}`);
  }

  return (
    <>
      <label className="row">
        <span className="grow">Bluetooth</span>
        <input type="checkbox" checked={s.on} onChange={(e) => commands.bluetoothSetOn(e.target.checked)} />
      </label>
      {s.on && s.devices.length === 0 && <p className="hint">No paired devices yet.</p>}
      {s.on &&
        s.devices.map((d) => (
          <div key={d.id} className="net-row">
            <span className="icon" aria-hidden>
              {d.audio ? HEADPHONES : BLUETOOTH}
            </span>
            <span className="grow">
              <span className="tile-label">{d.name}</span>
              <span className="module-desc">
                {busy === d.id ? (d.connected ? "Disconnecting…" : "Connecting…") : d.connected ? "Connected" : "Not connected"}
              </span>
            </span>
            {/* Windows lets apps connect audio devices only. */}
            {d.audio && (
              <button className={d.connected ? "" : "primary"} disabled={busy !== null} onClick={() => connect(d.id, d.name, !d.connected)}>
                {d.connected ? "Disconnect" : "Connect"}
              </button>
            )}
          </div>
        ))}
      {error && <p className="error">{error}</p>}
      <button className="link-row" onClick={() => commands.openSettings("bluetooth")}>
        <span className="grow">Pair a new device</span>
        <span className="icon" aria-hidden>
          {CHEVRON}
        </span>
      </button>
    </>
  );
}

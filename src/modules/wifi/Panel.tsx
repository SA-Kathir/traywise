import { useEffect, useState } from "react";
import { commands, events, type WifiNetwork } from "../../bindings";
import { Tile } from "../../shell/Tile";
import { useLive } from "../../shell/useLive";
import type { TileProps } from "..";

// Segoe Fluent Icons glyphs: Wi-Fi on, off (with a cross), and 1–3 bars.
const WIFI = String.fromCodePoint(0xe701);
const WIFI_OFF = String.fromCodePoint(0xeb5e);
const BARS = [0xe872, 0xe872, 0xe873, 0xe874, 0xe701].map((c) => String.fromCodePoint(c));
const LOCK = String.fromCodePoint(0xe72e);
const CHEVRON = String.fromCodePoint(0xe76c);
const CLOSE = String.fromCodePoint(0xe711);

const useWifi = () => useLive(commands.wifiState, events.wifiChanged);

/** Home tile: Wi-Fi on or off, and the network it's on (at 2×2 with its band). */
export function Tiles({ size, open }: TileProps) {
  const s = useWifi();
  if (!s) return null;
  const name = s.connected && (s.connected.ssid || "Connected");
  // 1×1: the name, as in the design (the icon shows on or off).
  const detail =
    size === "1x1"
      ? "Wi-Fi"
      : s.on === null
      ? "Not available"
      : !s.on
        ? "Off"
        : !name
          ? "Not connected"
          : size === "2x2" && s.connected?.band
            ? `${name} · ${s.connected.band}`
            : name;
  return (
    <Tile
      size={size}
      icon={s.on ? WIFI : WIFI_OFF}
      label="Wi-Fi"
      detail={detail}
      on={s.on === true}
      onToggle={s.on === null ? undefined : () => commands.wifiSetOn(!s.on)}
      onOpen={open}
    />
  );
}

/** The page: the switch, the network in use, and the others in range to join. */
export default function Panel() {
  const s = useWifi();
  const [scanning, setScanning] = useState(false);
  // The network a password is being typed for.
  const [joining, setJoining] = useState<string | null>(null);
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Look for networks each time the page opens.
  useEffect(() => {
    setScanning(true);
    commands.wifiScan().finally(() => setScanning(false));
  }, []);

  if (!s) return null;
  if (s.on === null) return <p className="hint">This PC has no Wi-Fi.</p>;

  async function connect(ssid: string, withPassword: string | null) {
    setBusy(ssid);
    setError(null);
    const r = await commands.wifiConnect(ssid, withPassword);
    setBusy(null);
    if (r.status === "error") {
      setError(`${ssid}: ${r.error}`);
      if (r.error === "wrong password") setJoining(ssid);
    } else {
      setJoining(null);
      setPassword("");
    }
  }

  // A secured network Windows hasn't saved asks for its password first.
  function pick(n: WifiNetwork) {
    if (n.secure && !n.saved) {
      setJoining(n.ssid);
      setPassword("");
    } else connect(n.ssid, null);
  }

  return (
    <>
      <label className="row">
        <span className="grow">Wi-Fi</span>
        <input type="checkbox" checked={s.on} onChange={(e) => commands.wifiSetOn(e.target.checked)} />
      </label>
      {s.on && s.namesHidden && (
        <div className="row">
          <p className="hint grow">Windows hides network names while location is off for desktop apps.</p>
          <button onClick={() => commands.openSettings("location")}>Location settings</button>
        </div>
      )}
      {s.on && s.connected && (
        <div className="net-row">
          <span className="icon" aria-hidden>
            {BARS[s.connected.bars]}
          </span>
          <span className="grow">
            <span className="tile-label">{s.connected.ssid || "Connected"}</span>
            <span className="module-desc">Connected{s.connected.band ? ` · ${s.connected.band}` : ""}</span>
          </span>
          <button onClick={() => commands.wifiDisconnect()}>Disconnect</button>
        </div>
      )}
      {s.on &&
        s.networks.map((n) =>
          joining === n.ssid ? (
            <form
              key={n.ssid}
              className="net-row"
              onSubmit={(e) => {
                e.preventDefault();
                connect(n.ssid, password);
              }}
            >
              <span className="grow">
                <span className="tile-label">{n.ssid}</span>
                <input
                  type="password"
                  className="text"
                  placeholder="Password"
                  aria-label={`Password for ${n.ssid}`}
                  autoFocus
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                />
              </span>
              <button type="button" className="icon-button" aria-label="Cancel" title="Cancel" onClick={() => setJoining(null)}>
                {CLOSE}
              </button>
              <button type="submit" className="primary" disabled={!password || busy === n.ssid}>
                {busy === n.ssid ? "Joining…" : "Join"}
              </button>
            </form>
          ) : (
            <button key={n.ssid} className="link-row net-row" disabled={busy !== null} onClick={() => pick(n)}>
              <span className="icon" aria-hidden>
                {BARS[n.bars]}
              </span>
              <span className="grow">
                <span className="tile-label">{n.ssid}</span>
                <span className="module-desc">
                  {busy === n.ssid ? "Connecting…" : [n.saved ? "Saved" : n.secure ? "Secured" : "Open", n.band].filter(Boolean).join(" · ")}
                </span>
              </span>
              {n.secure && (
                <span className="icon" aria-hidden>
                  {LOCK}
                </span>
              )}
            </button>
          ),
        )}
      {s.on && scanning && <p className="hint">Looking for networks…</p>}
      {error && <p className="error">{error}</p>}
      <button className="link-row" onClick={() => commands.openSettings("wifi")}>
        <span className="grow">More Wi-Fi settings</span>
        <span className="icon" aria-hidden>
          {CHEVRON}
        </span>
      </button>
    </>
  );
}

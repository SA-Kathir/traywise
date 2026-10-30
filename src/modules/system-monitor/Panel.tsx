import { commands, events, type Battery } from "../../bindings";
import { Num } from "../../shell/Tile";
import { useLive } from "../../shell/useLive";
import type { Size, TileProps } from "..";

const useSample = () => useLive(commands.systemMonitorState, events.systemSampled);

/** Home tiles, as in the design: CPU or Memory, a dot-matrix number and its graph; at 4×1 and 4×2
 *  both side by side. Clicking opens the page, which has network, battery and disk too. */
export function Tiles({ kind, size, open }: TileProps) {
  const s = useSample();
  const ready = s !== null && s.memoryTotal > 0;
  const history = ready ? s.history : [];
  const cpu = (
    <Reading
      label="CPU"
      sub={ready && s.cpuMhz !== null ? `${(s.cpuMhz / 1000).toFixed(1)} GHz` : ""}
      value={ready && s.cpu !== null ? String(s.cpu) : "–"}
      unit="%"
      values={history.map((p) => p.cpu)}
      floor={CPU_FLOOR}
      size={size}
    />
  );
  const memory = (
    <Reading
      label={size.startsWith("4") ? "RAM" : "Memory"}
      sub={ready ? `of ${Math.round(s.memoryTotal / 1024)} GB` : ""}
      value={ready ? (s.memoryUsed / 1024).toFixed(1) : "–"}
      unit="GB"
      values={history.map((p) => p.memory)}
      floor={MEMORY_FLOOR}
      size={size}
    />
  );
  return (
    <button className={`tile stats size-${size}`} onClick={open} title="Show details">
      {kind === "memory" ? memory : cpu}
      {kind === "monitor" && memory}
    </button>
  );
}

/** One reading on a tile: its name (and at 2×2 and 4×2 a detail, like the clock speed), the number
 *  in dot type with its unit, and from 2×1 up its graph. */
function Reading(props: { label: string; sub: string; value: string; unit: string; values: number[]; floor: number; size: Size }) {
  const tall = props.size === "2x2" || props.size === "4x2";
  return (
    <span className="reading">
      <span className="reading-text">
        <span className="reading-head">
          <span className="stat-chip-label">{props.label}</span>
          {tall && <span className="reading-sub">{props.sub}</span>}
        </span>
        <span className="stat-chip-value">
          <Num text={props.value} />
          {props.unit}
        </span>
      </span>
      {props.size !== "1x1" && (
        <span className="reading-graph">
          <Graph values={props.values} floor={props.floor} cols={tall ? 19 : 7} rows={tall ? 5 : 6} />
        </span>
      )}
    </span>
  );
}

export default function Panel() {
  const s = useSample();

  // Empty until the first sample after the panel opens.
  if (!s || s.memoryTotal === 0) return null;

  return (
    <>
      <Stat label="CPU" value={s.cpu === null ? "–" : `${s.cpu}%`}>
        <Graph values={s.history.map((p) => p.cpu)} floor={CPU_FLOOR} />
      </Stat>
      <Stat label="Memory" value={`${gib(s.memoryUsed)} of ${gib(s.memoryTotal)}`} />
      <Stat
        label="Network"
        value={s.download === null || s.upload === null ? "–" : `↓ ${rate(s.download)}  ↑ ${rate(s.upload)}`}
      >
        <Graph values={s.history.map((p) => p.download + p.upload)} floor={NET_FLOOR} />
      </Stat>
      {s.battery && <Stat label="Battery" value={battery(s.battery)} />}
      <Stat label="Disk" value={`${gib(s.diskFree)} free of ${gib(s.diskTotal)}`} />
    </>
  );
}

function Stat({ label, value, children }: { label: string; value: string; children?: React.ReactNode }) {
  return (
    <div className="stat">
      <div className="row">
        <span className="grow hint">{label}</span>
        <span className="stat-value">{value}</span>
      </div>
      {children}
    </div>
  );
}

/** The graphs zoom in to the highest reading shown, but never further than this (CPU %, network
 *  bytes a second), so a quiet PC's jitter doesn't look like a spike. */
const CPU_FLOOR = 10;
const MEMORY_FLOOR = 10;
const NET_FLOOR = 10 * 1024;
/** The Dot look's dots: 4 px with 3 px gaps. */
const PITCH = 7;

/** The last 60 seconds, newest on the right, scaled to the highest reading (or `floor`): a line,
 *  and for the Dot look a dot matrix of `cols` × `rows` (each style hides the other). */
function Graph({ values, floor, cols = 40, rows = 5 }: { values: number[]; floor: number; cols?: number; rows?: number }) {
  if (values.length < 2) return <svg className="graph" aria-hidden />;
  const top = Math.max(floor, ...values) * 1.1; // a little headroom above the peak
  // 60 seconds across 59 steps, newest on the right.
  const points = values.map((v, i) => `${59 - (values.length - 1 - i)},${20 - (v / top) * 20}`).join(" ");
  // Each column is the highest reading in its slice of the minute; columns before the first reading stay dark.
  const per = 60 / cols;
  const dots = Array.from({ length: cols }, (_, c) => {
    const slice = values.slice(Math.max(0, values.length - (cols - c) * per), values.length - (cols - 1 - c) * per);
    const lit = slice.length === 0 ? 0 : Math.max(1, Math.round((Math.max(...slice) / top) * rows));
    return Array.from({ length: rows }, (_, r) => (
      <circle
        key={r}
        cx={c * PITCH + PITCH / 2}
        cy={(rows - 1 - r) * PITCH + PITCH / 2}
        r={2}
        className={r >= lit ? "off" : c === cols - 1 ? "now" : undefined}
      />
    ));
  });
  return (
    <>
      <svg className="graph" viewBox="0 0 59 20" preserveAspectRatio="none" aria-hidden>
        <polyline points={points} />
      </svg>
      <svg className="graph-dots" viewBox={`0 0 ${cols * PITCH} ${rows * PITCH}`} aria-hidden>
        {dots}
      </svg>
    </>
  );
}

const gib = (mib: number) => `${(mib / 1024).toFixed(1)} GB`;

function rate(bytesPerSecond: number) {
  if (bytesPerSecond < 1024) return `${bytesPerSecond} B/s`;
  if (bytesPerSecond < 1024 * 1024) return `${(bytesPerSecond / 1024).toFixed(0)} KB/s`;
  return `${(bytesPerSecond / 1024 / 1024).toFixed(1)} MB/s`;
}

function battery({ percent, charging, secondsLeft }: Battery) {
  if (charging) return `${percent}%, charging`;
  if (secondsLeft === null) return `${percent}%`;
  const minutes = Math.round(secondsLeft / 60);
  const h = Math.floor(minutes / 60);
  return `${percent}%, ${h > 0 ? `${h} h ${minutes % 60} min` : `${minutes} min`} left`;
}

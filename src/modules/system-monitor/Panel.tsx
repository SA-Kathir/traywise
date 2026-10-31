import { commands, events, type Battery, type Point, type Reading as ReadingId, type SystemSample } from "../../bindings";
import { useSettings } from "../../shell/SettingsContext";
import { Num } from "../../shell/Tile";
import { useLive } from "../../shell/useLive";
import type { Size, TileProps } from "..";

const useSample = () => useLive(commands.systemMonitorState, events.systemSampled);

/** The graphs zoom in to the highest reading shown, but never further than this (CPU %, network
 *  bytes a second), so a quiet PC's jitter doesn't look like a spike. */
const CPU_FLOOR = 10;
const NET_FLOOR = 10 * 1024;
/** The Dot look's dots: 4 px with 3 px gaps. */
const PITCH = 7;

/** Home tiles, as in the design: CPU or Memory, a dot-matrix number and its graph; at 4×1 and 4×2
 *  two of them side by side, picked on the page. Clicking opens the page. */
export function Tiles({ kind, size, open }: TileProps) {
  const s = useSample();
  const { settings } = useSettings();
  const ready = s !== null && s.memoryTotal > 0;
  const reading = (id: ReadingId) => {
    const r = READINGS[id];
    return (
      <Reading
        key={id}
        label={size.startsWith("4") && id === "memory" ? "RAM" : r.label}
        sub={ready ? r.sub(s) : ""}
        value={ready ? r.value(s) : "–"}
        unit={r.unit}
        values={ready ? s.history.map(r.history) : []}
        floor={r.floor}
        size={size}
      />
    );
  };
  return (
    <button className={`tile stats size-${size}`} onClick={open} title="Show details">
      {kind === "monitor" ? settings.monitorReadings.map(reading) : reading(kind as ReadingId)}
    </button>
  );
}

/** What each reading shows on a tile: its name, a detail (at 2×2 and 4×2), the number (with its
 *  unit, apart for the dot type) and its history for the graph. */
const READINGS: Record<
  ReadingId,
  { label: string; short: string; unit: string; floor: number; sub: (s: SystemSample) => string; value: (s: SystemSample) => string; history: (p: Point) => number }
> = {
  cpu: {
    label: "CPU",
    short: "CPU",
    unit: "%",
    floor: CPU_FLOOR,
    sub: (s) => (s.cpuMhz === null ? "" : `${(s.cpuMhz / 1000).toFixed(1)} GHz`),
    value: (s) => (s.cpu === null ? "–" : String(s.cpu)),
    history: (p) => p.cpu,
  },
  memory: {
    label: "Memory",
    short: "RAM",
    unit: "GB",
    floor: 10,
    sub: (s) => `of ${Math.round(s.memoryTotal / 1024)} GB`,
    value: (s) => (s.memoryUsed / 1024).toFixed(1),
    history: (p) => p.memory,
  },
  download: {
    label: "Download",
    short: "Down",
    unit: "",
    floor: NET_FLOOR,
    sub: () => "",
    value: (s) => (s.download === null ? "–" : rate(s.download)),
    history: (p) => p.download,
  },
  upload: {
    label: "Upload",
    short: "Up",
    unit: "",
    floor: NET_FLOOR,
    sub: () => "",
    value: (s) => (s.upload === null ? "–" : rate(s.upload)),
    history: (p) => p.upload,
  },
};

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
      <TilePicker />
    </>
  );
}

/** Which two readings the 4×1 and 4×2 tile shows: a row each for its left and right. */
function TilePicker() {
  const { settings, save } = useSettings();
  return (
    <div className="tile-picker">
      <h2>Wide tile</h2>
      {([0, 1] as const).map((slot) => (
        <div key={slot} className="row">
          <span className="picker-side">{slot === 0 ? "Left" : "Right"}</span>
          <div className="segmented grow" role="radiogroup" aria-label={slot === 0 ? "Left reading" : "Right reading"}>
            {(Object.keys(READINGS) as ReadingId[]).map((id) => {
              const on = settings.monitorReadings[slot] === id;
              return (
                <button
                  key={id}
                  role="radio"
                  aria-checked={on}
                  aria-label={READINGS[id].label}
                  className={on ? "on" : ""}
                  onClick={() => {
                    const readings: [ReadingId, ReadingId] = [...settings.monitorReadings];
                    readings[slot] = id;
                    save({ ...settings, monitorReadings: readings });
                  }}
                >
                  {READINGS[id].short}
                </button>
              );
            })}
          </div>
        </div>
      ))}
    </div>
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

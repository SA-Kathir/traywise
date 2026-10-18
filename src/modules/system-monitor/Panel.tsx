import { useEffect, useState } from "react";
import { commands, events, type Battery, type SystemSample } from "../../bindings";

export default function Panel() {
  const [s, setSample] = useState<SystemSample | null>(null);

  useEffect(() => {
    let live = false; // an event beat the first fetch: don't overwrite it with older data
    const unlisten = events.systemSampled.listen((e) => {
      live = true;
      setSample(e.payload);
    });
    commands.systemMonitorState().then((s) => live || setSample(s));
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);

  // Empty until the first sample after the panel opens.
  if (!s || s.memoryTotal === 0) return null;

  return (
    <>
      <Stat label="CPU" value={s.cpu === null ? "–" : `${s.cpu}%`}>
        <Graph values={s.history.map((p) => p.cpu)} max={100} />
      </Stat>
      <Stat label="Memory" value={`${gib(s.memoryUsed)} of ${gib(s.memoryTotal)}`} />
      <Stat
        label="Network"
        value={s.download === null || s.upload === null ? "–" : `↓ ${rate(s.download)}  ↑ ${rate(s.upload)}`}
      >
        <Graph values={s.history.map((p) => p.download + p.upload)} />
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

/** A line over the last 60 seconds. `max` defaults to the highest value shown. */
function Graph({ values, max }: { values: number[]; max?: number }) {
  if (values.length < 2) return <svg className="graph" aria-hidden />;
  const top = max ?? Math.max(1, ...values);
  // 60 seconds across 59 steps, newest on the right.
  const points = values.map((v, i) => `${59 - (values.length - 1 - i)},${20 - (v / top) * 20}`).join(" ");
  return (
    <svg className="graph" viewBox="0 0 59 20" preserveAspectRatio="none" aria-hidden>
      <polyline points={points} />
    </svg>
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

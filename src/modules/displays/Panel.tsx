import type { CSSProperties } from "react";
import { commands, events, type Display, type Project } from "../../bindings";
import { Slider } from "../../shell/Slider";
import { Num, Tile, TileCard } from "../../shell/Tile";
import { useLive } from "../../shell/useLive";
import type { TileProps } from "..";

// Segoe Fluent Icons glyphs.
const SUN = String.fromCodePoint(0xe706);
const LAPTOP = String.fromCodePoint(0xe7f8);
const MONITOR = String.fromCodePoint(0xe7f4);

const PROJECTS: { project: Project; label: string; short: string; icon: string }[] = [
  { project: "pcOnly", label: "PC screen only", short: "PC only", icon: LAPTOP },
  { project: "duplicate", label: "Duplicate", short: "Duplicate", icon: String.fromCodePoint(0xe8c8) }, // "Copy"
  { project: "extend", label: "Extend", short: "Extend", icon: String.fromCodePoint(0xe740) }, // "Full Screen"
  { project: "secondOnly", label: "Second screen only", short: "2nd only", icon: MONITOR },
];

const useDisplays = () => useLive(commands.displaysState, events.displaysChanged);

const setBrightness = (d: Display) => (v: number) => commands.displaysSetBrightness(d.id, v);

/** Home tiles: brightness (1×1 the whole tile fills to it; 2×1 and 4×1 a slider for the first screen
 *  that can change; 4×2 every screen) and Project, like Win+P. */
export function Tiles({ kind, size, open }: TileProps) {
  const state = useDisplays();
  if (!state) return null;
  if (kind === "project") return <ProjectTile project={state.project} size={size} open={open} />;
  const main = state.displays.find((d) => d.brightness !== null);
  if (!main) return <Tile size={size} icon={SUN} label="Brightness" detail="Can't change" onOpen={open} />;
  const level = main.brightness!;
  if (size === "1x1")
    return (
      <button className="tile vol size-1x1" style={{ "--level": `${level}%` } as CSSProperties} title={`Brightness: ${level}`} onClick={open}>
        <Num text={String(level)} />
        <span className="vol-icon" aria-hidden>
          {SUN}
        </span>
      </button>
    );
  const rows = size === "4x2" ? state.displays.filter((d) => d.brightness !== null).slice(0, 3) : [main];
  return (
    <div className={`tile vol size-${size}`}>
      <div className="vol-main">
        {rows.map((d, i) => (
          <div key={d.id} className="vol-app">
            <div className="vol-head">
              {i === 0 ? (
                <button className="tile-title" title="Details" onClick={open}>
                  <span className="tile-label">Brightness</span>
                  {size !== "2x1" && <span className="tile-detail">{d.name}</span>}
                </button>
              ) : (
                <span className="tile-detail">{d.name}</span>
              )}
              <Num text={String(d.brightness)} />
            </div>
            <div className="vol-bar">
              <Slider name={d.name} what="brightness" value={d.brightness!} onChange={setBrightness(d)} bare />
              <span className="vol-icon" aria-hidden>
                {SUN}
              </span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function ProjectTile({ project, size, open }: { project: Project | null; size: TileProps["size"]; open: () => void }) {
  const current = PROJECTS.find((p) => p.project === project);
  if (size === "2x1" && current)
    return (
      <TileCard size={size} label="Project" detail={current.short} onOpen={open}>
        <Choices project={current.project} icons />
      </TileCard>
    );
  const next = current && PROJECTS[(PROJECTS.indexOf(current) + 1) % PROJECTS.length];
  return (
    <Tile
      size={size}
      icon={current?.icon ?? LAPTOP}
      label="Project"
      detail={current ? current.short : "One screen"}
      onToggle={next && (() => commands.displaysSetProject(next.project))}
      onOpen={open}
    />
  );
}

/** The four Win+P choices: icons only on the 2×1 tile, a 2×2 grid with names on the page. */
function Choices({ project, icons }: { project: Project; icons?: boolean }) {
  return (
    <div className={`segmented${icons ? "" : " project-grid"}`} role="radiogroup" aria-label="Project">
      {PROJECTS.map((p) => (
        <button
          key={p.project}
          role="radio"
          aria-checked={project === p.project}
          className={project === p.project ? "on" : ""}
          title={p.label}
          onClick={() => commands.displaysSetProject(p.project)}
        >
          <span className="icon" aria-hidden>
            {p.icon}
          </span>
          {!icons && p.label}
        </button>
      ))}
    </div>
  );
}

export default function Panel() {
  const state = useDisplays();
  if (!state) return null;
  return (
    <>
      {state.displays.map((d) => (
        <div key={d.id} className="mixer-row">
          <span className="app-icon glyph" aria-hidden>
            {d.internal ? LAPTOP : MONITOR}
          </span>
          <span className="mixer-name" title={d.name}>
            {d.name}
          </span>
          {d.brightness === null ? (
            <span className="hint">Can't change</span>
          ) : (
            <Slider name={d.name} what="brightness" value={d.brightness} onChange={setBrightness(d)} />
          )}
        </div>
      ))}
      {state.displays.some((d) => !d.internal && d.brightness === null) && (
        <p className="hint">To change a monitor's brightness from here, turn on DDC/CI in the monitor's own menu.</p>
      )}
      <h2>Project</h2>
      {state.project ? <Choices project={state.project} /> : <p className="hint">Connect a second screen to choose how they're used.</p>}
    </>
  );
}

import type { ReactNode } from "react";
import { commands, events, type Display, type Project } from "../../bindings";
import { DialTile } from "../../shell/Dial";
import { Slider } from "../../shell/Slider";
import { Num, Tile, TileCard } from "../../shell/Tile";
import { useLive } from "../../shell/useLive";
import type { TileProps } from "..";

// Segoe Fluent Icons glyphs.
const SUN = String.fromCodePoint(0xe706);
const LAPTOP = String.fromCodePoint(0xe7f8);
const MONITOR = String.fromCodePoint(0xe7f4);

/** The Project icons, drawn: laptop, two overlapping screens, two side by side, a desktop monitor.
 *  The stroke stays 1.3 px at every size (non-scaling, see `.project-icon` in liquid.css). */
const SHAPES: Record<Project, ReactNode> = {
  pcOnly: (
    <>
      <rect x="3.5" y="1.5" width="13" height="8.5" rx="1.5" />
      <path d="M1 12.5h18" />
    </>
  ),
  duplicate: (
    <>
      <rect x="7.5" y="1" width="10" height="7" rx="1.2" />
      <rect x="2.5" y="5.5" width="10" height="7" rx="1.2" />
    </>
  ),
  extend: (
    <>
      <rect x="1" y="3" width="8" height="8" rx="1.5" />
      <rect x="11" y="3" width="8" height="8" rx="1.5" />
    </>
  ),
  secondOnly: (
    <>
      <rect x="2.5" y="1" width="15" height="9.5" rx="1.5" />
      <path d="M10 10.5V13M6.5 13h7" />
    </>
  ),
};
const icon = (p: Project) => (
  <svg className="project-icon" viewBox="0 0 20 14" fill="none" stroke="currentColor" strokeWidth={1.3} strokeLinecap="round" aria-hidden>
    {SHAPES[p]}
  </svg>
);

/** `label` is the tooltip, `name` the tile and page, `note` the 4×2 tile's line under it. */
const PROJECTS: { project: Project; label: string; name: string; note: string; icon: ReactNode }[] = [
  { project: "pcOnly", label: "PC screen only", name: "PC only", note: "Only this PC's screen is on", icon: icon("pcOnly") },
  { project: "duplicate", label: "Duplicate", name: "Duplicate", note: "Both screens show the same thing", icon: icon("duplicate") },
  { project: "extend", label: "Extend", name: "Extend", note: "The second screen adds more room", icon: icon("extend") },
  { project: "secondOnly", label: "Second screen only", name: "2nd only", note: "Only the second screen is on", icon: icon("secondOnly") },
];

const useDisplays = () => useLive(commands.displaysState, events.displaysChanged);

const setBrightness = (d: Display) => (v: number) => commands.displaysSetBrightness(d.id, v);

/** Home tiles, as in the design: brightness like the volume tile (1×1 the whole tile fills to it;
 *  2×1, 4×1 and 2×2 the first screen that can change; 4×2 every screen) and Project. */
export function Tiles({ kind, size, open }: TileProps) {
  const state = useDisplays();
  if (!state) return null;
  if (kind === "project") return <ProjectTile project={state.project} size={size} open={open} />;
  const main = state.displays.find((d) => d.brightness !== null);
  if (!main) return <Tile size={size} icon={SUN} label="Brightness" detail="Can't change" onOpen={open} />;
  const level = main.brightness!;
  if (size === "1x1" || size === "2x2")
    return (
      <DialTile size={size} noun="Brightness" icon={SUN} level={level} detail={main.name} onChange={setBrightness(main)} open={open} />
    );
  const changeable = state.displays.filter((d) => d.brightness !== null);
  // 4×2 with one screen: the old 2×2 layout, wider, rather than one row and a gap.
  const big = size === "4x2" && changeable.length === 1;
  if (size !== "4x2" || big)
    return (
      <div className={`tile vol size-${size}`}>
        <div className="vol-main">
          <div className="vol-head">
            <button className="tile-title" title="Details" onClick={open}>
              <span className="tile-label">Brightness</span>
              {size === "4x1" && <span className="tile-detail">{main.name}</span>}
            </button>
            {!big && <Num text={String(level)} />}
          </div>
          {big && (
            <>
              <span className="vol-big">
                <Num text={String(level)} />
              </span>
              <span className="tile-detail">{main.name}</span>
            </>
          )}
          <Bar display={main} />
        </div>
      </div>
    );
  // 4×2: a row per screen (three fit).
  return (
    <div className="tile vol size-4x2">
      <div className="vol-main">
        <div className="vol-head">
          <button className="tile-title" title="Details" onClick={open}>
            <span className="tile-label">Brightness</span>
          </button>
        </div>
        {changeable
          .slice(0, 3)
          .map((d) => (
            <div key={d.id} className="vol-app">
              <div className="vol-head">
                <span className="tile-detail" title={d.name}>
                  <span className="icon" aria-hidden>
                    {d.internal ? LAPTOP : MONITOR}
                  </span>{" "}
                  {d.name}
                </span>
                <Num text={String(d.brightness)} />
              </div>
              <Bar display={d} />
            </div>
          ))}
      </div>
    </div>
  );
}

/** The volume tile's bar: the fill is the thumb, with the sun inside it. */
function Bar({ display }: { display: Display }) {
  return (
    <div className="vol-bar">
      <Slider name={display.name} what="brightness" value={display.brightness!} onChange={setBrightness(display)} bare />
      <span className="vol-icon" aria-hidden>
        {SUN}
      </span>
    </div>
  );
}

function ProjectTile({ project, size, open }: { project: Project | null; size: TileProps["size"]; open: () => void }) {
  const current = PROJECTS.find((p) => p.project === project);
  const shown = current ?? PROJECTS[0];
  if (size === "1x1")
    return <Tile size={size} icon={shown.icon} label="Project" detail={current ? current.name : "1 screen"} onOpen={open} />;
  if (size === "2x1") {
    // Shows the current mode (the dots) and opens the page, where the choices are.
    return (
      <TileCard
        size={size}
        label="Project"
        detail={
          <span className="project-pips" aria-hidden>
            {PROJECTS.map((p) => (
              <i key={p.project} className={p === current ? "on" : ""} />
            ))}
          </span>
        }
      >
        <button className="project-now" title="Details" onClick={open}>
          <span className="icon" aria-hidden>
            {shown.icon}
          </span>
          <span className="project-name">{current ? current.name : "One screen"}</span>
        </button>
      </TileCard>
    );
  }
  return (
    <TileCard size={size} label="Project" detail={current ? undefined : "One screen"}>
      {size === "4x2" && (
        <div className="project-now">
          <span className="project-preview icon" aria-hidden>
            {shown.icon}
          </span>
          <span className="project-text">
            <span className="project-name">{current ? current.name : "One screen"}</span>
            <span className="tile-detail">{current ? current.note : "Connect a second screen to switch."}</span>
          </span>
        </div>
      )}
      <Choices project={project} tile />
    </TileCard>
  );
}

/** The four Project choices: on a tile, icon above the name; on the page, icon beside it.
 *  With one screen (no `project`) they're dimmed and can't be picked. */
function Choices({ project, tile }: { project: Project | null; tile?: boolean }) {
  return (
    <div className={`segmented project-grid${tile ? " stacked" : ""}${project ? "" : " unavailable"}`} role="radiogroup" aria-label="Project">
      {PROJECTS.map((p) => (
        <button
          key={p.project}
          role="radio"
          aria-checked={project === p.project}
          className={project === p.project ? "on" : ""}
          title={p.label}
          disabled={!project}
          onClick={() => commands.displaysSetProject(p.project)}
        >
          <span className="icon" aria-hidden>
            {p.icon}
          </span>
          {p.name}
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
        <div key={d.id} className="display-row">
          <div className="row">
            <span className="icon" aria-hidden>
              {d.internal ? LAPTOP : MONITOR}
            </span>
            <span className="grow" title={d.name}>
              {d.name}
            </span>
            {d.brightness === null ? <span className="tile-detail">Can't change</span> : <Num text={String(d.brightness)} />}
          </div>
          {d.brightness !== null && <Bar display={d} />}
        </div>
      ))}
      {state.displays.some((d) => !d.internal && d.brightness === null) && (
        <p className="hint">Turn on DDC/CI in a monitor's own menu to change it here.</p>
      )}
      {state.project && (
        <>
          <h2>Project</h2>
          <Choices project={state.project} />
        </>
      )}
    </>
  );
}

import type { CSSProperties, ReactNode } from "react";
import { commands, events, type Display, type Project } from "../../bindings";
import { Slider } from "../../shell/Slider";
import { Num, Tile, TileCard } from "../../shell/Tile";
import { useLive } from "../../shell/useLive";
import type { TileProps } from "..";

// Segoe Fluent Icons glyphs.
const SUN = String.fromCodePoint(0xe706);
const LAPTOP = String.fromCodePoint(0xe7f8);
const MONITOR = String.fromCodePoint(0xe7f4);

/** The design's Project icons: two screens side by side; a screen that's off is four corner dots. */
const Screen = ({ x, on, wide }: { x: number; on: boolean; wide?: boolean }) =>
  on ? (
    <rect x={x + 0.5} y={2.5} width={wide ? 19 : 8} height={7} rx={1.2} fill="none" stroke="currentColor" />
  ) : (
    <g fill="currentColor">
      {[
        [x + 1, 3],
        [x + 7, 3],
        [x + 1, 9],
        [x + 7, 9],
      ].map(([cx, cy]) => (
        <circle key={`${cx}${cy}`} cx={cx} cy={cy} r={0.9} />
      ))}
    </g>
  );
const icon = (left: boolean, right: boolean, wide = false) => (
  <svg className="project-icon" viewBox="0 0 20 12" aria-hidden>
    {wide ? <Screen x={0} on wide /> : <Screen x={0} on={left} />}
    {!wide && <Screen x={11} on={right} />}
    {left && right && !wide && <path d="M4.5 6h0M15.5 6h0" stroke="currentColor" strokeWidth={2} strokeLinecap="round" />}
  </svg>
);

/** `label` is the tooltip, `page` the System page's button, `short` the 1×1 tile (about seven letters). */
const PROJECTS: { project: Project; label: string; page: string; short: string; icon: ReactNode }[] = [
  { project: "pcOnly", label: "PC screen only", page: "PC only", short: "PC only", icon: icon(true, false) },
  { project: "duplicate", label: "Duplicate", page: "Duplicate", short: "Mirror", icon: icon(true, true) },
  { project: "extend", label: "Extend", page: "Extend", short: "Extend", icon: icon(true, true, true) },
  { project: "secondOnly", label: "Second screen only", page: "2nd only", short: "2nd only", icon: icon(false, true) },
];

const useDisplays = () => useLive(commands.displaysState, events.displaysChanged);

const setBrightness = (d: Display) => (v: number) => commands.displaysSetBrightness(d.id, v);

/** Home tiles, as in the design: brightness like the volume tile (1×1 the whole tile fills to it;
 *  2×1, 4×1 and 2×2 the first screen that can change; 4×2 every screen) and Project, like Win+P. */
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
  const changeable = state.displays.filter((d) => d.brightness !== null);
  // 4×2 with one screen: the 2×2 layout, wider, rather than one row and a gap.
  const big = size === "2x2" || (size === "4x2" && changeable.length === 1);
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
  if (size === "2x1")
    return (
      <TileCard size={size} label="Project" onOpen={open}>
        <Choices project={current?.project ?? null} icons />
      </TileCard>
    );
  const next = current && PROJECTS[(PROJECTS.indexOf(current) + 1) % PROJECTS.length];
  return (
    <Tile
      size={size}
      icon={(current ?? PROJECTS[0]).icon}
      label="Project"
      detail={current ? current.short : "Single"}
      onToggle={next && (() => commands.displaysSetProject(next.project))}
      onOpen={open}
    />
  );
}

/** The four Win+P choices: icons only on the 2×1 tile, a 2×2 grid with names on the page.
 *  With one screen (no `project`) they're dimmed and can't be picked. */
function Choices({ project, icons }: { project: Project | null; icons?: boolean }) {
  return (
    <div className={`segmented${icons ? " icons" : " project-grid"}${project ? "" : " unavailable"}`} role="radiogroup" aria-label="Project">
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
          {!icons && p.page}
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

import type { CSSProperties, ReactNode } from "react";
import type { Size } from "../modules";

// The two building blocks of every tile on the home board. Modules draw their tiles with these,
// so the board's look lives in a few classes in liquid.css (.tile, .tile-card and the --tile-*
// variables) and can be restyled in one place.

// Segoe Fluent Icons glyph.
const CHEVRON = String.fromCodePoint(0xe76c);

type TileProps = {
  size: Size;
  /** A Segoe Fluent Icons glyph, or an icon element. */
  icon: ReactNode;
  label: string;
  /** One short line under the label: the state or a value. */
  detail: string;
  /** Lit up, like a switched-on Windows quick setting. Leave out for tiles that aren't on/off. */
  on?: boolean;
  /** The tile's main action, like switching it on or off. Without one, clicking opens the details. */
  onToggle?: () => void;
  /** Opens the module's own page. Without one there's no chevron. */
  onOpen?: () => void;
  /** A level from 0 to 100 (volume): the Dot look fills the tile up to it. */
  level?: number;
};

/** A one-click tile, like Windows' quick settings. At 1×1 it's the icon and the detail only (the
 *  name is in its tooltip), with no chevron. */
export function Tile({ size, icon, label, detail, on, onToggle, onOpen, level }: TileProps) {
  const small = size === "1x1";
  return (
    <div
      className={`tile size-${size}${on ? " on" : ""}${level === undefined ? "" : " level"}`}
      style={level === undefined ? undefined : ({ "--level": `${level}%` } as CSSProperties)}
    >
      <button
        className="tile-main"
        aria-pressed={on}
        aria-label={small ? `${label}: ${detail}` : undefined}
        title={small ? `${label}: ${detail}` : undefined}
        onClick={onToggle ?? onOpen}
      >
        <span className="tile-icon" aria-hidden>
          {icon}
        </span>
        {!small && <span className="tile-label">{label}</span>}
        <span className="tile-detail" title={small ? undefined : detail}>
          <Num text={detail} />
        </span>
      </button>
      {onOpen && !small && (
        <button className="tile-more" aria-label={`${label} details`} title="Details" onClick={onOpen}>
          {CHEVRON}
        </button>
      )}
    </div>
  );
}

/** A tile with controls in it (sliders, choices, readings): a title line, then its contents. */
export function TileCard({
  size,
  label,
  detail,
  onOpen,
  children,
}: {
  size: Size;
  label: string;
  /** Shown at the right of the title line. */
  detail?: string;
  /** Opens the module's own page: the title line becomes a link to it. */
  onOpen?: () => void;
  children: ReactNode;
}) {
  const title = (
    <>
      <span className="tile-label grow">{label}</span>
      {detail && <span className="tile-detail">{detail}</span>}
      {onOpen && (
        <span className="icon" aria-hidden>
          {CHEVRON}
        </span>
      )}
    </>
  );
  return (
    <div className={`tile tile-card size-${size}`}>
      {onOpen ? (
        <button className="tile-title" title="Details" onClick={onOpen}>
          {title}
        </button>
      ) : (
        <div className="tile-title">{title}</div>
      )}
      {children}
    </div>
  );
}

/** A reading with its digits in dot type (the Dot look) and its unit beside them in the UI font:
 *  "23%" is 23 then %. Text that doesn't start with a number stays as it is. */
export function Num({ text }: { text: string }) {
  const [, digits, rest] = /^([\d.:×]+)(.*)$/s.exec(text) ?? [];
  if (!digits) return text;
  return (
    <>
      <span className="num">{digits}</span>
      {rest}
    </>
  );
}

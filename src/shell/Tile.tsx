// Segoe Fluent Icons glyph.
const CHEVRON = String.fromCodePoint(0xe76c);

type TileProps = {
  /** A Segoe Fluent Icons glyph. */
  icon: string;
  label: string;
  /** One short line under the label: the state or a value. */
  detail: string;
  /** Lit up, like a switched-on Windows quick setting. Leave out for tiles that aren't on/off. */
  on?: boolean;
  /** The tile's main action, like switching it on or off. Without one, clicking opens the details. */
  onToggle?: () => void;
  /** Opens the module's own page. Without one there's no chevron. */
  onOpen?: () => void;
};

/** One square on the home view, half the panel wide. */
export function Tile({ icon, label, detail, on, onToggle, onOpen }: TileProps) {
  return (
    <div className={on ? "tile on" : "tile"}>
      <button className="tile-main" aria-pressed={on} onClick={onToggle ?? onOpen}>
        <span className="tile-icon" aria-hidden>
          {icon}
        </span>
        <span className="tile-label">{label}</span>
        <span className="tile-detail" title={detail}>
          {detail}
        </span>
      </button>
      {onOpen && (
        <button className="tile-more" aria-label={`${label} details`} title="Details" onClick={onOpen}>
          {CHEVRON}
        </button>
      )}
    </div>
  );
}

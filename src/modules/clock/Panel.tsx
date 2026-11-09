import { useEffect, useState } from "react";
import type { ClockStyle } from "../../bindings";
import { Row } from "../../shell/Settings";
import { Num } from "../../shell/Tile";
import { useSettings } from "../../shell/SettingsContext";
import type { TileProps } from "..";
import { city, clockAt, dayShift, difference, differenceLabel, hands, isNight, time, toNextMinute } from "./time";
import { ZONES, matches, place } from "./zones";

// Segoe Fluent Icons glyphs.
const MOON = String.fromCodePoint(0xe708);
const SUN = String.fromCodePoint(0xe706);
const CHEVRON_DOWN = String.fromCodePoint(0xe70d);
const CHEVRON_UP = String.fromCodePoint(0xe70e);

/** The current time, redrawn as each minute starts, and only while the panel is open: hidden, the
 *  page is too (WebView2 is made invisible), so the timer stops and it catches up when shown. */
function useMinute() {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    let timer: number | undefined;
    const tick = () => {
      const date = new Date();
      setNow(date);
      timer = window.setTimeout(tick, toNextMinute(date.getTime()));
    };
    const visibility = () => {
      clearTimeout(timer);
      if (!document.hidden) tick();
    };
    visibility();
    document.addEventListener("visibilitychange", visibility);
    return () => {
      clearTimeout(timer);
      document.removeEventListener("visibilitychange", visibility);
    };
  }, []);
  return now;
}

const DATE = {
  medium: { weekday: "short", day: "numeric", month: "short" }, // Sat 31 Oct
  long: { weekday: "long", day: "numeric", month: "long" }, // Saturday 31 October
} satisfies Record<string, Intl.DateTimeFormatOptions>;

/** "Sat 31" (always that order: some locales put the day first), "Sat Oct 31" or "Saturday, October 31". */
function day(date: Date, format: keyof typeof DATE | "short", zone?: string) {
  const part = (options: Intl.DateTimeFormatOptions) => date.toLocaleDateString([], { ...options, timeZone: zone });
  if (format === "short") return `${part({ weekday: "short" })} ${part({ day: "numeric" })}`;
  return format === "medium" ? part(DATE.medium).replace(",", "") : part(DATE.long);
}

/** "21:40", or "9:40" with a small "PM": the digits in dot type. `stacked`: the hours over the minutes. */
function Time({ date, zone, stacked }: { date: Date; zone?: string; stacked?: boolean }) {
  const { digits, period } = time(date, zone);
  const [hours, minutes] = digits.split(":");
  const end = period && <span className="clock-period">{period}</span>;
  return stacked ? (
    <span className="clock-time stacked">
      {/* "01" over "46": two digits on each line, so they line up. */}
      <Num text={hours.padStart(2, "0")} />
      <span>
        <Num text={minutes} />
        {end}
      </span>
    </span>
  ) : (
    <span className="clock-time">
      <Num text={digits} />
      {end}
    </span>
  );
}

/** 24 dots, one per hour of the day; those gone by are lit, this hour in the accent. */
function HourDots({ hour }: { hour: number }) {
  return (
    <svg className="clock-hours" viewBox="0 0 120 4" aria-hidden>
      {Array.from({ length: 24 }, (_, i) => (
        <circle key={i} cx={2.5 + i * 5} cy={2} r={1.6} className={i < hour ? "lit" : i === hour ? "now" : undefined} />
      ))}
    </svg>
  );
}

const TICKS = 60;

/** A round face of dots: 60 ticks (each fifth brighter) and the hands as rows of dots, the minute
 *  hand's tip in the accent. */
function Face({ date, zone }: { date: Date; zone?: string }) {
  const { hours, minutes } = clockAt(date, zone);
  const angle = hands(hours, minutes);
  const at = (degrees: number, r: number) => {
    const a = (degrees * Math.PI) / 180;
    return { cx: 50 + r * Math.sin(a), cy: 50 - r * Math.cos(a) };
  };
  const hand = (degrees: number, dots: number, tip: boolean) =>
    Array.from({ length: dots }, (_, i) => (
      <circle key={`${dots}-${i}`} {...at(degrees, (i + 1) * 5)} r={2.2} className={tip && i === dots - 1 ? "tip" : "hand"} />
    ));
  const line = (degrees: number, from: number, to: number, className: string) => {
    const a = at(degrees, from);
    const b = at(degrees, to);
    return <line key={`${className}-${degrees}-${to}`} x1={a.cx} y1={a.cy} x2={b.cx} y2={b.cy} className={className} />;
  };
  return (
    <svg className="clock-face" viewBox="0 0 100 100" aria-hidden>
      <g className="face-dots">
        {Array.from({ length: TICKS }, (_, i) => (
          <circle key={i} {...at(i * 6, 45)} r={i % 5 ? 1.3 : 2} className={i % 5 ? "tick" : "tick major"} />
        ))}
        <circle cx={50} cy={50} r={2.2} className="hand" />
        {hand(angle.hour, 4, false)}
        {hand(angle.minute, 7, true)}
      </g>
      {/* The Glass look draws the same time as lines instead: fine ticks, solid hands. */}
      <g className="face-lines">
        {Array.from({ length: TICKS }, (_, i) =>
          i % 5 ? line(i * 6, 43, 46, "ltick") : line(i * 6, 40.5, 46.5, "lmajor"),
        )}
        {line(angle.hour, 0, 24, "lhand")}
        {line(angle.minute, 0, 36, "lminute")}
        <circle cx={50} cy={50} r={3} className="lcap" />
      </g>
    </svg>
  );
}

/** The second zone, always in its own inset well so it never reads as this PC's time: the city,
 *  then its time. `full` (4×2): a column, with its date, the
 *  hours between, a moon or sun, and a Tomorrow / Yesterday tag. */
function Zone({ date, zone, full }: { date: Date; zone: string; full?: boolean }) {
  const shift = dayShift(date, zone);
  if (!full)
    return (
      <span className="clock-well">
        <span className="clock-over clock-city">{place(zone)}</span>
        <Time date={date} zone={zone} />
      </span>
    );
  return (
    <span className="clock-well full">
      <span className="clock-well-head">
        <span className="clock-over clock-city">{place(zone)}</span>
        <span className="clock-glyph" aria-label={isNight(date, zone) ? "Night" : "Day"}>
          {isNight(date, zone) ? MOON : SUN}
        </span>
      </span>
      <span className="clock-over dim">
        {day(date, "short", zone)} · {differenceLabel(difference(date, zone))}
      </span>
      <span className="clock-well-foot">
        {shift !== 0 && <span className="clock-tag">{shift > 0 ? "Tomorrow" : "Yesterday"}</span>}
        <Time date={date} zone={zone} />
      </span>
    </span>
  );
}

/** This PC's place, named like the second zone's, so two times on one tile never read as one. */
const HERE = city(Intl.DateTimeFormat().resolvedOptions().timeZone);

function Label({ zone, extra }: { zone?: string; extra?: string }) {
  return (
    <span className="clock-label">
      <span className="clock-over">
        {zone ? place(zone) : HERE}
        {extra && ` · ${extra}`}
      </span>
    </span>
  );
}

/** Analogue with a second zone: two matching faces side by side, each named under it. */
function Twins({ date, zone, size }: { date: Date; zone: string; size: TileProps["size"] }) {
  return (
    <span className="clock-twins">
      <span className="clock-place">
        <Face date={date} />
        <span className="clock-place-text">
          <Label />
          <span className="clock-over dim">{day(date, "short")}</span>
        </span>
      </span>
      <span className="clock-place">
        <Face date={date} zone={zone} />
        <span className="clock-place-text">
          <Label zone={zone} />
          <span className="clock-over dim">
            {day(date, "short", zone)}
            {size === "4x2" && ` · ${differenceLabel(difference(date, zone))}`}
          </span>
        </span>
      </span>
    </span>
  );
}

/** Home tile, Digital or Analogue (picked on the page). Bigger sizes add the date; with a second
 *  zone set, 4×1 and up show it in its well. A click opens the page. */
export function Tiles({ size, open }: TileProps) {
  const now = useMinute();
  const { clockZone, clockStyle } = useSettings().settings;
  const zoned = !!clockZone && size !== "1x1" && size !== "2x1";
  return (
    <div className={`tile clock size-${size} ${clockStyle}${zoned ? " zoned" : ""}`}>
      <button className="clock-main" title="Clock" onClick={open}>
        {clockStyle === "analogue" && zoned ? (
          <Twins date={now} zone={clockZone} size={size} />
        ) : (
          <>
            {clockStyle === "analogue" ? analogue(now, size) : digital(now, size, zoned)}
            {zoned && <Zone date={now} zone={clockZone} full={size === "4x2"} />}
          </>
        )}
      </button>
    </div>
  );
}

/** `zoned`: the second zone's box shares the tile, so this PC's time takes less room. */
function digital(now: Date, size: TileProps["size"], zoned: boolean) {
  if (size === "1x1")
    return (
      <span className="clock-here">
        <Time date={now} stacked />
        <span className="clock-over">{day(now, "short")}</span>
      </span>
    );
  if (size === "2x2" && zoned)
    return (
      <span className="clock-here">
        <Label extra={day(now, "short")} />
        <Time date={now} />
      </span>
    );
  // 2×2 on its own: the hours over the minutes, large, over a dot for each hour of the day.
  if (size === "2x2")
    return (
      <span className="clock-here">
        <span className="clock-over">{day(now, "medium")}</span>
        <Time date={now} stacked />
        <HourDots hour={now.getHours()} />
      </span>
    );
  // 4×2 with a zone: laid out like the zone's column beside it (place, date, then the time).
  if (size === "4x2" && zoned)
    return (
      <span className="clock-here">
        <Label />
        <span className="clock-over dim">{day(now, "medium")}</span>
        <Time date={now} />
      </span>
    );
  if (size === "4x2")
    return (
      <span className="clock-here">
        <span className="clock-over">{day(now, "long")}</span>
        <Time date={now} />
        <span className="clock-over dim">{HERE}</span>
      </span>
    );
  // 4×1 with a zone: room for the place only.
  if (zoned)
    return (
      <span className="clock-here">
        <Label />
        <Time date={now} />
      </span>
    );
  return (
    <span className="clock-here">
      <Time date={now} />
      <span className="clock-over">{day(now, "medium")}</span>
    </span>
  );
}

/** Analogue without a second zone (with one, it's `Twins`). */
function analogue(now: Date, size: TileProps["size"]) {
  // 2×2: the day and the time in small digits above a large face.
  if (size === "2x2")
    return (
      <span className="clock-here clock-stack">
        <span className="clock-head">
          <span className="clock-over">{day(now, "short")}</span>
          <span className="clock-over">{[time(now).digits, time(now).period].join(" ").trim()}</span>
        </span>
        <Face date={now} />
      </span>
    );
  if (size === "1x1") return <Face date={now} />;
  // 4×2: the face with the time and date beside it, as on the page.
  if (size === "4x2")
    return (
      <>
        <Face date={now} />
        <span className="clock-here">
          <Time date={now} />
          <span className="clock-over">{day(now, "long")}</span>
        </span>
      </>
    );
  return (
    <>
      <Face date={now} />
      <span className="clock-here">
        <Time date={now} />
        <span className="clock-over">{day(now, size === "4x1" ? "medium" : "short")}</span>
      </span>
    </>
  );
}

const STYLES: { style: ClockStyle; label: string }[] = [
  { style: "digital", label: "Digital" },
  { style: "analogue", label: "Analogue" },
];

export default function Panel() {
  const now = useMinute();
  const { settings, save } = useSettings();
  const zone = settings.clockZone;
  return (
    <>
      <div className="clock-page">
        {settings.clockStyle === "analogue" && <Face date={now} />}
        <span className="clock-here">
          <Time date={now} />
          <span className="clock-over">{day(now, "long")}</span>
        </span>
      </div>
      <Row label="Style">
        <div className="segmented" role="radiogroup" aria-label="Clock style">
          {STYLES.map(({ style, label }) => (
            <button
              key={style}
              role="radio"
              aria-checked={settings.clockStyle === style}
              className={settings.clockStyle === style ? "on" : ""}
              onClick={() => save({ ...settings, clockStyle: style })}
            >
              {label}
            </button>
          ))}
        </div>
      </Row>
      <Row
        label="Second time zone"
        desc={zone ? `${place(zone)}: ${differenceLabel(difference(now, zone))}. On the 4×1, 2×2 and 4×2 tiles.` : "Shown on the 4×1, 2×2 and 4×2 tiles."}
        stacked
      >
        <ZonePicker zone={zone} onPick={(clockZone) => save({ ...settings, clockZone })} />
      </Row>
    </>
  );
}

/** The places, A to Z by country. */
const SORTED = [...ZONES].sort((a, b) => a[1].localeCompare(b[1]));

/** The second zone: a button showing the one picked, which opens a list of places with a search
 *  box ready to type in ("ind" leaves India; Enter picks the first left, Esc closes). */
function ZonePicker({ zone, onPick }: { zone: string | null; onPick: (zone: string | null) => void }) {
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState("");
  const known = ZONES.find(([z]) => z === zone);
  const pick = (z: string | null) => {
    onPick(z);
    setOpen(false);
    setSearch("");
  };
  const found = SORTED.filter((entry) => matches(entry, search));
  return (
    <div className="zone-picker">
      <button className="zone-current" aria-expanded={open} onClick={() => setOpen(!open)}>
        <span className="grow">{zone ? (known ? `${known[1]} · ${place(zone)}` : place(zone)) : "None"}</span>
        <span className="icon" aria-hidden>
          {open ? CHEVRON_UP : CHEVRON_DOWN}
        </span>
      </button>
      {open && (
        <input
          className="text zone-search"
          placeholder="Search a country or city"
          aria-label="Search places"
          autoFocus
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && found[0]) pick(found[0][0]);
            if (e.key === "Escape") {
              e.stopPropagation(); // close the list, not the panel
              setOpen(false);
              setSearch("");
            }
          }}
        />
      )}
      {open && (
        <div className="segmented vertical zone-list" role="listbox" aria-label="Second time zone">
          {!search && (
            <button role="option" aria-selected={!zone} className={!zone ? "on" : ""} onClick={() => pick(null)}>
              None
            </button>
          )}
          {found.length === 0 && <p className="hint">No places match.</p>}
          {found.map(([z, country, cities]) => (
            <button key={z} role="option" aria-selected={zone === z} className={zone === z ? "on" : ""} onClick={() => pick(z)}>
              <span className="grow">{country}</span>
              <span className="zone-cities">{cities}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

// The clock's time-zone and format logic: plain functions, tested in time.test.ts. `zone` is an IANA
// time zone like "Europe/London"; leaving it out means this PC's own.

/** Milliseconds until the clock next needs redrawing: the start of the next minute. */
export const toNextMinute = (now: number) => 60000 - (now % 60000);

/** The time as the PC's locale writes it, split into its digits ("2:05", "14:05") and its AM/PM
 *  ("PM", or "" in 24-hour locales), so the digits can be drawn in dot type. */
export function time(date: Date, zone?: string, locale?: string) {
  const parts = new Intl.DateTimeFormat(locale, { hour: "numeric", minute: "2-digit", timeZone: zone }).formatToParts(date);
  const digits = parts
    .filter((p) => p.type === "hour" || p.type === "minute" || (p.type === "literal" && p.value.trim()))
    .map((p) => p.value)
    .join("");
  return { digits, period: parts.find((p) => p.type === "dayPeriod")?.value ?? "" };
}

/** A zone's wall clock at `date`, as if it were UTC, to the minute. */
function wall(date: Date, zone?: string) {
  const p: Record<string, string> = {};
  const format = new Intl.DateTimeFormat("en-US", {
    timeZone: zone,
    hourCycle: "h23",
    year: "numeric",
    month: "numeric",
    day: "numeric",
    hour: "numeric",
    minute: "numeric",
  });
  for (const { type, value } of format.formatToParts(date)) p[type] = value;
  return Date.UTC(+p.year, +p.month - 1, +p.day, +p.hour, +p.minute);
}

/** How far `zone` is ahead of `local` at `date`, in minutes (negative if behind). */
export const difference = (date: Date, zone: string, local?: string) => (wall(date, zone) - wall(date, local)) / 60000;

/** Whether it's already tomorrow (1) or still yesterday (-1) in `zone`, compared with `local`. */
export function dayShift(date: Date, zone: string, local?: string) {
  const day = (t: number) => Math.floor(t / 86400000);
  return Math.sign(day(wall(date, zone)) - day(wall(date, local)));
}

/** Whether it's night in `zone` (6 pm to 6 am), for its moon or sun. */
export const isNight = (date: Date, zone: string) => {
  const hour = new Date(wall(date, zone)).getUTCHours();
  return hour < 6 || hour >= 18;
};

/** Where an analogue clock's hands point, in degrees clockwise from 12. */
export const hands = (hours: number, minutes: number) => ({ hour: ((hours % 12) + minutes / 60) * 30, minute: minutes * 6 });

/** A difference in minutes as "+5:30 h", "−8 h" or "Same time". */
export function differenceLabel(minutes: number) {
  if (minutes === 0) return "Same time";
  const m = Math.abs(minutes);
  const rest = m % 60;
  return `${minutes > 0 ? "+" : "−"}${Math.floor(m / 60)}${rest ? `:${String(rest).padStart(2, "0")}` : ""} h`;
}

/** "America/Argentina/Buenos_Aires" -> "Buenos Aires". */
export const city = (zone: string) => zone.slice(zone.lastIndexOf("/") + 1).replace(/_/g, " ");

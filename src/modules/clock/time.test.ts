// Run with `pnpm test`. Fixed zones and dates, so it passes whatever zone this PC is in.
import { ZONES, place } from "./zones.ts";
import { city, dayShift, difference, differenceLabel, hands, isNight, time, toNextMinute } from "./time.ts";

function check(name: string, got: unknown, expected: unknown) {
  if (JSON.stringify(got) !== JSON.stringify(expected)) throw new Error(`${name}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(got)}`);
}

const at = new Date("2026-10-31T13:05:30Z");
check("next minute", toNextMinute(at.getTime()), 30000);
check("on the minute", toNextMinute(Date.UTC(2026, 9, 31, 13, 5)), 60000);

check("24-hour", time(at, "Europe/London", "en-GB"), { digits: "13:05", period: "" });
check("12-hour", time(at, "America/New_York", "en-US"), { digits: "9:05", period: "AM" });

check("ahead", difference(at, "Asia/Kolkata", "Europe/London"), 330);
check("behind", difference(at, "America/Los_Angeles", "Europe/London"), -420);
// 1 November 2026: London is back on GMT, New York not yet off daylight saving.
check("DST gap", difference(new Date("2026-11-01T03:00:00Z"), "America/New_York", "Europe/London"), -240);

check("tomorrow", dayShift(new Date("2026-10-31T20:00:00Z"), "Asia/Tokyo", "Europe/London"), 1);
check("yesterday", dayShift(new Date("2026-10-31T02:00:00Z"), "America/Los_Angeles", "Europe/London"), -1);
check("same day", dayShift(at, "Asia/Kolkata", "Europe/London"), 0);

check("night", isNight(at, "Asia/Tokyo"), true); // 22:05 there
check("day", isNight(at, "Europe/London"), false); // 13:05
check("hands at 21:40", hands(21, 40), { hour: 290, minute: 240 });
check("hands at noon", hands(12, 0), { hour: 0, minute: 0 });

check("label ahead", differenceLabel(330), "+5:30 h");
check("label behind", differenceLabel(-420), "−7 h");
check("label same", differenceLabel(0), "Same time");

check("city", city("America/Argentina/Buenos_Aires"), "Buenos Aires");
check("city plain", city("UTC"), "UTC");

for (const [zone] of ZONES) new Intl.DateTimeFormat("en", { timeZone: zone }); // throws if unknown
check("zones unique", new Set(ZONES.map(([z]) => z)).size, ZONES.length);
check("place", place("Asia/Kolkata"), "Delhi");
check("place, unlisted", place("America/Argentina/Cordoba"), "Cordoba");

console.log("clock: all checks passed");

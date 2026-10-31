// The places a second time zone can be: one per time zone people commonly need, named by the
// cities people know (Windows' full list has hundreds, many named after small towns).
import { city } from "./time.ts";

/** [time zone, country or region, its cities]. The first city names it on the tile. */
export const ZONES: [zone: string, country: string, cities: string][] = [
  ["Pacific/Honolulu", "Hawaii", "Honolulu"],
  ["America/Anchorage", "Alaska", "Anchorage"],
  ["America/Los_Angeles", "US Pacific", "Los Angeles, San Francisco, Seattle"],
  ["America/Vancouver", "Canada Pacific", "Vancouver"],
  ["America/Denver", "US Mountain", "Denver"],
  ["America/Phoenix", "Arizona", "Phoenix"],
  ["America/Chicago", "US Central", "Chicago, Dallas, Houston"],
  ["America/Mexico_City", "Mexico", "Mexico City"],
  ["America/New_York", "US Eastern", "New York, Washington, Miami"],
  ["America/Toronto", "Canada Eastern", "Toronto, Montreal"],
  ["America/Bogota", "Colombia", "Bogotá"],
  ["America/Lima", "Peru", "Lima"],
  ["America/Santiago", "Chile", "Santiago"],
  ["America/Argentina/Buenos_Aires", "Argentina", "Buenos Aires"],
  ["America/Sao_Paulo", "Brazil", "São Paulo, Rio de Janeiro"],
  ["Atlantic/Reykjavik", "Iceland", "Reykjavík"],
  ["Europe/London", "UK", "London, Manchester, Edinburgh"],
  ["Europe/Dublin", "Ireland", "Dublin"],
  ["Europe/Lisbon", "Portugal", "Lisbon"],
  ["Europe/Madrid", "Spain", "Madrid, Barcelona"],
  ["Europe/Paris", "France", "Paris"],
  ["Europe/Amsterdam", "Netherlands", "Amsterdam"],
  ["Europe/Berlin", "Germany", "Berlin, Munich, Frankfurt"],
  ["Europe/Zurich", "Switzerland", "Zurich, Geneva"],
  ["Europe/Rome", "Italy", "Rome, Milan"],
  ["Europe/Stockholm", "Sweden", "Stockholm"],
  ["Europe/Warsaw", "Poland", "Warsaw"],
  ["Europe/Athens", "Greece", "Athens"],
  ["Europe/Kyiv", "Ukraine", "Kyiv"],
  ["Europe/Istanbul", "Turkey", "Istanbul"],
  ["Europe/Moscow", "Russia", "Moscow"],
  ["Africa/Lagos", "Nigeria", "Lagos"],
  ["Africa/Cairo", "Egypt", "Cairo"],
  ["Africa/Johannesburg", "South Africa", "Johannesburg, Cape Town"],
  ["Africa/Nairobi", "Kenya", "Nairobi"],
  ["Asia/Jerusalem", "Israel", "Jerusalem, Tel Aviv"],
  ["Asia/Riyadh", "Saudi Arabia", "Riyadh"],
  ["Asia/Tehran", "Iran", "Tehran"],
  ["Asia/Dubai", "UAE", "Dubai, Abu Dhabi"],
  ["Asia/Karachi", "Pakistan", "Karachi, Lahore"],
  ["Asia/Kolkata", "India", "Delhi, Mumbai, Bengaluru, Chennai"],
  ["Asia/Colombo", "Sri Lanka", "Colombo"],
  ["Asia/Kathmandu", "Nepal", "Kathmandu"],
  ["Asia/Dhaka", "Bangladesh", "Dhaka"],
  ["Asia/Bangkok", "Thailand", "Bangkok"],
  ["Asia/Ho_Chi_Minh", "Vietnam", "Ho Chi Minh City, Hanoi"],
  ["Asia/Jakarta", "Indonesia", "Jakarta"],
  ["Asia/Singapore", "Singapore", "Singapore"],
  ["Asia/Kuala_Lumpur", "Malaysia", "Kuala Lumpur"],
  ["Asia/Manila", "Philippines", "Manila"],
  ["Asia/Shanghai", "China", "Beijing, Shanghai"],
  ["Asia/Hong_Kong", "Hong Kong", "Hong Kong"],
  ["Asia/Taipei", "Taiwan", "Taipei"],
  ["Asia/Seoul", "South Korea", "Seoul"],
  ["Asia/Tokyo", "Japan", "Tokyo, Osaka"],
  ["Australia/Perth", "Australia West", "Perth"],
  ["Australia/Adelaide", "Australia Central", "Adelaide"],
  ["Australia/Brisbane", "Queensland", "Brisbane"],
  ["Australia/Sydney", "Australia East", "Sydney, Melbourne, Canberra"],
  ["Pacific/Auckland", "New Zealand", "Auckland, Wellington"],
];

/** A zone's name on the tile: its first city ("Delhi" for Asia/Kolkata), or for a zone not in the
 *  list (from an older version), the city in its id. */
export function place(zone: string) {
  const known = ZONES.find(([z]) => z === zone);
  return known ? known[2].split(",")[0] : city(zone);
}

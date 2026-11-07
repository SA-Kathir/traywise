// Run with `pnpm test`.
import { DOTS, sandAt } from "./hourglass.ts";

function check(name: string, ok: boolean) {
  if (!ok) throw new Error(`hourglass: ${name}`);
}

const total = 1500;
const bottomStart = DOTS.length - 46;
const start = sandAt(0, total);
check("93 dots", DOTS.length === 93);
check("starts full on top, empty below", start.slice(0, 46).every((s) => s === "sand" || s === "dim") && start.slice(bottomStart).every((s) => s === ""));
const end = sandAt(total, total);
check("ends with a full pile and an empty top", end.slice(bottomStart).every((s) => s === "pile") && end.slice(0, 46).every((s) => s === ""));

let piled = new Set<number>();
let sandBefore = 46;
for (let t = 0; t <= total; t++) {
  const now = sandAt(t, total);
  const pile = new Set(now.flatMap((s, i) => (s === "pile" ? [i] : [])));
  for (const i of piled) check(`a landed dot stays (t=${t})`, pile.has(i));
  piled = pile;
  now.forEach((s, i) => s === "grain" && check(`grains stay below the neck (t=${t})`, DOTS[i].r > 6));
  const sand = now.slice(0, 46).filter((s) => s !== "").length;
  check(`the top only empties (t=${t})`, sand <= sandBefore);
  sandBefore = sand;
}
check("a one-minute phase still ends full", sandAt(60, 60).slice(bottomStart).every((s) => s === "pile"));
console.log("hourglass ok");

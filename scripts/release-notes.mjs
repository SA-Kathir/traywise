// Prints a release's notes for its GitHub release: its section of CHANGELOG.md, then the install
// note. Fails if CHANGELOG.md has no section for that version, so no release goes out without one.
// Usage: node scripts/release-notes.mjs v0.2.0
import { readFileSync } from "node:fs";

const version = (process.argv[2] ?? "").replace(/^v/, "");
const changelog = readFileSync(new URL("../CHANGELOG.md", import.meta.url), "utf8").replace(/\r\n/g, "\n");
// A section starts at "## 0.2.0 (date)" and runs to the next "## ".
const start = changelog.search(new RegExp(`^## ${version.replace(/\./g, "\\.")}\\b.*$`, "m"));
if (!version || start < 0) {
  console.error(`CHANGELOG.md has no "## ${version}" section`);
  process.exit(1);
}
const body = changelog.slice(changelog.indexOf("\n", start) + 1);
const end = body.search(/^## /m);
console.log((end < 0 ? body : body.slice(0, end)).trim());
console.log(`
---

**Install:** download \`Traywise_${version}_x64-setup.exe\` below. Windows may warn that the installer is unrecognised (it isn't code-signed yet): click **More info**, then **Run anyway**. Already have Traywise? It offers this update itself.`);

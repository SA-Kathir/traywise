import { useState } from "react";
import { commands, events } from "../bindings";
import { useLive } from "./useLive";

/** "Version X is available" with an Install button. Renders nothing until Rust has found an update. */
export function UpdateNotice() {
  const version = useLive(commands.updateAvailable, events.updateAvailable);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  if (!version) return null;

  async function install() {
    setInstalling(true);
    setError(null);
    // On success Traywise quits here and the installer restarts it.
    const r = await commands.updateInstall();
    if (r.status === "error") setError(r.error);
    setInstalling(false);
  }

  return (
    <section>
      <div className="row">
        <span className="grow">Traywise {version} is available</span>
        <button disabled={installing} onClick={install}>
          {installing ? "Downloading…" : "Install & restart"}
        </button>
      </div>
      {error && <p className="error">{error}</p>}
    </section>
  );
}

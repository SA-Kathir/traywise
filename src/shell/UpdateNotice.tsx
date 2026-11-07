import { useEffect, useState } from "react";
import { commands, events } from "../bindings";
import { useSettings } from "./SettingsContext";
import { useLive } from "./useLive";

/** "Version X is available" with an Install button. Renders nothing until Rust has found an update. */
export function UpdateNotice() {
  const version = useLive(commands.updateAvailable, events.updateAvailable);
  const [installing, setInstalling] = useState(false);
  const [percent, setPercent] = useState(0);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    const unlisten = events.updateProgress.listen((e) => setPercent(e.payload));
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);
  if (!version) return null;

  async function install() {
    setInstalling(true);
    setError(null);
    setPercent(0);
    // On success Traywise quits here and the installer quietly starts the new version.
    const r = await commands.updateInstall();
    if (r.status === "error") setError(r.error);
    setInstalling(false);
  }

  return (
    <section className="notice">
      <div className="row">
        <span className="grow">Traywise {version} is available</span>
        <button disabled={installing} onClick={install}>
          {!installing ? "Install & restart" : percent >= 100 ? "Installing…" : `Downloading ${percent}%`}
        </button>
      </div>
      {installing && <progress className="update-progress" max={100} value={percent} />}
      {error && <p className="error">{error}</p>}
    </section>
  );
}

/** Once after an update: "Traywise updated to X", with a link to that release's notes. */
export function WhatsNew({ version }: { version: string }) {
  const { settings, save } = useSettings();
  if (!version || settings.lastSeenVersion === version || !settings.onboarded) return null;
  return (
    <section className="notice">
      <div className="row">
        <span className="grow">Traywise updated to {version}</span>
        <button onClick={() => commands.openReleaseNotes(version)}>See what's new</button>
        <button aria-label="Dismiss" title="Dismiss" onClick={() => save({ ...settings, lastSeenVersion: version })}>
          ✕
        </button>
      </div>
    </section>
  );
}

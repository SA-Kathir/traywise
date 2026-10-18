import { Suspense, useEffect, useState } from "react";
import { commands, events, type Settings } from "./bindings";
import { modules } from "./modules";
import { LiquidBackground } from "./shell/LiquidBackground";
import { SettingsPage } from "./shell/Settings";
import "./shell/liquid.css";

// Segoe Fluent Icons glyphs.
const GEAR = String.fromCodePoint(0xe713);
const BACK = String.fromCodePoint(0xe72b);

function App() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [view, setView] = useState<"home" | "settings">("home");

  useEffect(() => {
    commands.getSettings().then(setSettings);
  }, []);

  useEffect(() => {
    const unlisten = events.panelShown.listen(() => {
      setView("home"); // every open starts on the main view, like Windows' own flyouts
      // Report click-to-drawn time: two animation frames means the browser has really drawn.
      requestAnimationFrame(() => requestAnimationFrame(() => commands.panelPainted()));
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);

  // Settings arrive within milliseconds of startup, long before anyone opens the panel.
  if (!settings) return null;

  return (
    <main className="panel">
      <LiquidBackground preset={settings.preset} motion={settings.motion} />
      <header className="row">
        <h1 className="grow">{view === "settings" ? "Settings" : "Traywise"}</h1>
        <button
          className="icon-button"
          aria-label={view === "settings" ? "Back" : "Settings"}
          title={view === "settings" ? "Back" : "Settings"}
          onClick={() => setView(view === "settings" ? "home" : "settings")}
        >
          {view === "settings" ? BACK : GEAR}
        </button>
      </header>

      {view === "settings" ? (
        <SettingsPage settings={settings} onChange={setSettings} />
      ) : settings.enabledModules.length === 0 ? (
        <p className="hint">No modules are on. Turn them on in Settings.</p>
      ) : (
        settings.enabledModules.map((id) => {
          const { name, icon, Panel } = modules[id];
          return (
            <section key={id}>
              <h2>
                <span className="icon" aria-hidden>
                  {icon}
                </span>
                {name}
              </h2>
              <Suspense fallback={null}>
                <Panel />
              </Suspense>
            </section>
          );
        })
      )}
    </main>
  );
}

export default App;

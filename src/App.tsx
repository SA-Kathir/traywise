import { Suspense, useEffect, useState } from "react";
import { commands, events, type ModuleId, type Settings } from "./bindings";
import { modules } from "./modules";
import { LiquidBackground } from "./shell/LiquidBackground";
import { SettingsPage } from "./shell/Settings";
import "./shell/liquid.css";

// Segoe Fluent Icons glyphs.
const GEAR = String.fromCodePoint(0xe713);
const BACK = String.fromCodePoint(0xe72b);

function App() {
  const [settings, setSettings] = useState<Settings | null>(null);
  // The home view, Settings, or one module's own page.
  const [view, setView] = useState<"home" | "settings" | ModuleId>("home");

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

  // The registry's order, not the order modules were switched on in: tiles first, then rows.
  const enabled = (Object.keys(modules) as ModuleId[]).filter((id) => settings.enabledModules.includes(id));
  const title = view === "home" ? "Traywise" : view === "settings" ? "Settings" : modules[view].name;

  return (
    // The preset class goes here too, so cards and controls can use its colours (--accent).
    <main className={`panel bg-${settings.preset}`}>
      <LiquidBackground preset={settings.preset} motion={settings.motion} />
      <header className="row">
        {view !== "home" && (
          <button className="icon-button" aria-label="Back" title="Back" onClick={() => setView("home")}>
            {BACK}
          </button>
        )}
        <h1 className="grow">{title}</h1>
        {view === "home" && (
          <button className="icon-button" aria-label="Settings" title="Settings" onClick={() => setView("settings")}>
            {GEAR}
          </button>
        )}
      </header>

      {view === "settings" ? (
        <SettingsPage settings={settings} onChange={setSettings} />
      ) : view !== "home" ? (
        <ModulePage id={view} />
      ) : enabled.length === 0 ? (
        <p className="hint">No modules are on. Turn them on in Settings.</p>
      ) : (
        <div className="home">
          {enabled.map((id) => {
            const { Summary } = modules[id];
            return (
              <Suspense key={id} fallback={null}>
                <Summary open={() => setView(id)} />
              </Suspense>
            );
          })}
        </div>
      )}
    </main>
  );
}

function ModulePage({ id }: { id: ModuleId }) {
  const { Panel } = modules[id];
  return (
    <section>
      <Suspense fallback={null}>
        <Panel />
      </Suspense>
    </section>
  );
}

export default App;

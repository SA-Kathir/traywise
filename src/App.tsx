import { Suspense, useEffect, useRef, useState } from "react";
import { commands, events, type ModuleId, type Settings } from "./bindings";
import { modules } from "./modules";
import { LiquidBackground } from "./shell/LiquidBackground";
import { SettingsPage } from "./shell/Settings";
import { UpdateNotice } from "./shell/UpdateNotice";
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

  // The window is as tall as this, so tell Rust each time the content changes height.
  const panel = useRef<HTMLElement>(null);
  const ready = settings !== null;
  useEffect(() => {
    const el = panel.current;
    if (!el) return;
    const observer = new ResizeObserver(() => commands.panelFit(el.getBoundingClientRect().height, view === "home"));
    observer.observe(el);
    return () => observer.disconnect();
  }, [ready, view]);

  // Settings arrive within milliseconds of startup, long before anyone opens the panel.
  if (!settings) return null;

  // The registry's order, not the order modules were switched on in: tiles first, then rows.
  const enabled = (Object.keys(modules) as ModuleId[]).filter((id) => settings.enabledModules.includes(id));
  // The welcome screen shows the name large instead.
  const title = view === "home" ? (enabled.length > 0 ? "Traywise" : "") : view === "settings" ? "Settings" : modules[view].name;

  return (
    // The preset class goes here too, so cards and controls can use its colours (--accent).
    <main ref={panel} className={`panel bg-${settings.preset}`}>
      <LiquidBackground />
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

      <UpdateNotice />

      {view === "settings" ? (
        <SettingsPage settings={settings} onChange={setSettings} />
      ) : view !== "home" ? (
        <ModulePage id={view} />
      ) : enabled.length === 0 ? (
        <div className="welcome">
          <p className="welcome-name">Traywise</p>
          <p className="hint">Pick the modules you want, and they'll show up here.</p>
          <button onClick={() => setView("settings")}>Choose modules</button>
        </div>
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
  if (!Panel) return null;
  return (
    <section>
      <Suspense fallback={null}>
        <Panel />
      </Suspense>
    </section>
  );
}

export default App;

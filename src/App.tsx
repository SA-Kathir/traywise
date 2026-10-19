import { Suspense, useEffect, useState } from "react";
import { commands, events, type ModuleId, type Settings } from "./bindings";
import { categories, modules, type Category } from "./modules";
import { ModulesPage } from "./shell/ModulesPage";
import { SettingsPage } from "./shell/Settings";
import { Setup } from "./shell/Setup";
import { UpdateNotice } from "./shell/UpdateNotice";
import "./shell/liquid.css";

// Segoe Fluent Icons glyphs.
const HOME = String.fromCodePoint(0xe80f);
const GEAR = String.fromCodePoint(0xe713);
const POWER = String.fromCodePoint(0xe7e8);
const BACK = String.fromCodePoint(0xe72b);

function App() {
  const [settings, setSettings] = useState<Settings | null>(null);
  // Home, a category's tab, Settings, or the Modules page (opened from Settings).
  const [view, setView] = useState<"home" | Category | "settings" | "modules">("home");

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
  // Categories with a page to show. Quick toggles has none, so it only appears on Home.
  const tabs = (Object.keys(categories) as Category[]).filter((c) =>
    enabled.some((id) => modules[id].category === c && modules[id].Panel),
  );
  const tabbed = view !== "settings" && view !== "modules";
  // The welcome screen shows the name large instead.
  const title =
    view === "home"
      ? enabled.length > 0
        ? "Traywise"
        : ""
      : view === "settings"
        ? "Settings"
        : view === "modules"
          ? "Modules"
          : categories[view].name;

  // The look and preset classes go on the panel, so cards and controls can use their colours (--accent).
  const panelClass = `panel look-${settings.look} bg-${settings.preset}`;

  if (!settings.onboarded) {
    return (
      <main className={panelClass}>
        <Setup settings={settings} onChange={setSettings} />
      </main>
    );
  }

  return (
    <main className={panelClass}>
      <header className="row">
        {!tabbed && (
          <button
            className="icon-button"
            aria-label="Back"
            title="Back"
            onClick={() => setView(view === "modules" ? "settings" : "home")}
          >
            {BACK}
          </button>
        )}
        <h1 className="grow">{title}</h1>
        {tabbed && (
          <>
            <button className="icon-button" aria-label="Settings" title="Settings" onClick={() => setView("settings")}>
              {GEAR}
            </button>
            <button className="icon-button" aria-label="Quit" title="Quit Traywise" onClick={() => commands.quit()}>
              {POWER}
            </button>
          </>
        )}
      </header>

      {tabbed && tabs.length > 0 && (
        <nav className="tabs" aria-label="Categories">
          {(["home", ...tabs] as const).map((t) => (
            <button
              key={t}
              className={view === t ? "on" : ""}
              aria-label={t === "home" ? "Home" : categories[t].name}
              title={t === "home" ? "Home" : categories[t].name}
              onClick={() => setView(t)}
            >
              {t === "home" ? HOME : categories[t].icon}
            </button>
          ))}
        </nav>
      )}

      <UpdateNotice />

      {view === "settings" ? (
        <SettingsPage settings={settings} onChange={setSettings} openModules={() => setView("modules")} />
      ) : view === "modules" ? (
        <ModulesPage settings={settings} onChange={setSettings} />
      ) : view !== "home" ? (
        <CategoryPage ids={enabled.filter((id) => modules[id].category === view)} />
      ) : enabled.length === 0 ? (
        <div className="welcome">
          <p className="welcome-name">Traywise</p>
          <p className="hint">Pick the modules you want, and they'll show up here.</p>
          <button onClick={() => setView("modules")}>Choose modules</button>
        </div>
      ) : (
        <div className="home">
          {enabled.map((id) => {
            const { Summary } = modules[id];
            return (
              <Suspense key={id} fallback={null}>
                <Summary open={() => setView(modules[id].category)} />
              </Suspense>
            );
          })}
        </div>
      )}
    </main>
  );
}

/** A category's tab: the page of each of its modules that's on, one card each. */
function CategoryPage({ ids }: { ids: ModuleId[] }) {
  return ids.map((id) => {
    const { name, Panel } = modules[id];
    if (!Panel) return null;
    return (
      <section key={id}>
        <h2>{name}</h2>
        <Suspense fallback={null}>
          <Panel />
        </Suspense>
      </section>
    );
  });
}

export default App;

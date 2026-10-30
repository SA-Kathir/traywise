import { Suspense, useEffect, useState } from "react";
import { commands, events, type ModuleId, type Settings } from "./bindings";
import { categories, modules, type Category, type Size } from "./modules";
import { AddTiles } from "./shell/AddTiles";
import { add, pack, type Placed } from "./shell/board";
import { Board, boardToShow } from "./shell/HomeBoard";
import { ModulesPage } from "./shell/ModulesPage";
import { SettingsPage, useSave } from "./shell/Settings";
import { SettingsContext } from "./shell/SettingsContext";
import { Setup } from "./shell/Setup";
import { UpdateNotice } from "./shell/UpdateNotice";
import "./shell/liquid.css";
import "./shell/dot.css";

// Segoe Fluent Icons glyphs.
const HOME = String.fromCodePoint(0xe80f);
const GEAR = String.fromCodePoint(0xe713);
const POWER = String.fromCodePoint(0xe7e8);
const BACK = String.fromCodePoint(0xe72b);
const EDIT = String.fromCodePoint(0xe70f);

/** How long the mouse can be outside an edge-opened panel before it closes. */
const LEAVE_MS = 400;

function App() {
  const [settings, setSettings] = useState<Settings | null>(null);
  // Home, a category's tab, Settings, or the Modules page (opened from Settings).
  const [view, setView] = useState<"home" | Category | "settings" | "modules">("home");
  // Edit mode on Home, and its list of tiles to add.
  const [editing, setEditing] = useState(false);
  const [adding, setAdding] = useState(false);
  // Reset asks once more before it throws the layout away.
  const [confirmReset, setConfirmReset] = useState(false);

  useEffect(() => {
    commands.getSettings().then(setSettings);
  }, []);

  useEffect(() => {
    // Opened from the screen's edge: close it once the mouse has left (and not come back, or
    // clicked in, within LEAVE_MS).
    let leave: number | undefined;
    const left = () => {
      leave = window.setTimeout(() => document.hasFocus() || commands.hidePanel(), LEAVE_MS);
    };
    const back = () => clearTimeout(leave);
    const root = document.documentElement;
    const unlisten = events.panelShown.listen((e) => {
      const { backdrop, hover } = e.payload;
      // Transparent looks: how bright it is behind the panel (0–1), for the glass's strength.
      if (backdrop !== null) root.style.setProperty("--backdrop", String(backdrop / 100));
      clearTimeout(leave);
      root.removeEventListener("mouseleave", left);
      root.removeEventListener("mouseenter", back);
      if (hover) {
        root.addEventListener("mouseleave", left);
        root.addEventListener("mouseenter", back);
        // It slides in from the edge (once; not with Windows' animation effects off).
        if (!matchMedia("(prefers-reduced-motion: reduce)").matches)
          document.querySelector(".panel")?.animate([{ transform: "translateX(24px)", opacity: 0 }, {}], { duration: 220, easing: "cubic-bezier(0, 0, 0, 1)" });
      }
      setView("home"); // every open starts on the main view, like Windows' own flyouts
      setEditing(false);
      setAdding(false);
      // Report click-to-drawn time: two animation frames means the browser has really drawn.
      requestAnimationFrame(() => requestAnimationFrame(() => commands.panelPainted()));
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);

  // Esc steps out: from the tray of tiles to add, then out of edit mode.
  useEffect(() => {
    if (!editing) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") (adding ? setAdding : setEditing)(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [editing, adding]);

  // The look's classes: on the panel, so cards and controls can use its colours (--accent), and on
  // html, so the page behind the panel (the scrollbar's strip) gets the same background.
  const lookClass =
    settings &&
    `look-${settings.look} bg-${settings.preset} accent-${settings.accent}${settings.transparent ? " clear" : ""}${settings.singleHue ? " flat" : ""}${settings.plainBackground ? " plain" : ""}`;
  useEffect(() => {
    document.documentElement.className = lookClass ?? "";
  }, [lookClass]);

  const { save, error } = useSave({ settings: settings!, onChange: setSettings });

  // Settings arrive within milliseconds of startup, long before anyone opens the panel.
  if (!settings) return null;

  // The registry's order, not the order modules were switched on in.
  const enabled = (Object.keys(modules) as ModuleId[]).filter((id) => settings.enabledModules.includes(id));
  // Categories with a page to show. Quick toggles has none, so it only appears on Home.
  const tabs = (Object.keys(categories) as Category[]).filter((c) =>
    enabled.some((id) => modules[id].category === c && modules[id].Panel),
  );
  const auto = settings.autoArrange;
  const tiles = boardToShow(settings.board, enabled, auto);
  // Every change to the board is saved straight away.
  const setBoard = (board: Placed[]) => save({ ...settings, board });
  const editHome = view === "home" && editing;
  const tabbed = view !== "settings" && view !== "modules" && !editHome;
  // The welcome screen shows the name large instead.
  const title = editHome
    ? "Edit home"
    : view === "home"
      ? enabled.length > 0
        ? "Traywise"
        : ""
      : view === "settings"
        ? "Settings"
        : view === "modules"
          ? "Modules"
          : categories[view].name;

  const panelClass = `panel ${lookClass}`;

  if (!settings.onboarded) {
    return (
      <main className={panelClass}>
        <Setup settings={settings} onChange={setSettings} />
      </main>
    );
  }

  return (
    <SettingsContext.Provider value={{ settings, save }}>
    <main className={panelClass}>
      <header className="row">
        {!tabbed && !editHome && (
          <button
            className="icon-button"
            aria-label="Back"
            title="Back"
            onClick={() => setView(view === "modules" ? "settings" : "home")}
          >
            {BACK}
          </button>
        )}
        {editHome ? (
          <div className="grow">
            <h1>{title}</h1>
            <p className="hint">Drag to move, corner to resize</p>
          </div>
        ) : (
          <h1 className={view === "home" && enabled.length > 0 ? "grow wordmark" : "grow"}>{title}</h1>
        )}
        {editHome && (
          <button className="primary" onClick={() => setEditing(false)}>
            Done
          </button>
        )}
        {tabbed && (
          <>
            {view === "home" && enabled.length > 0 && (
              <button className="icon-button" aria-label="Edit home" title="Edit home" onClick={() => setEditing(true)}>
                {EDIT}
              </button>
            )}
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

      {editHome && (
        <div className="row edit-bar">
          <label className="row grow">
            <span className="grow">Auto-arrange</span>
            <input
              type="checkbox"
              checked={auto}
              onChange={(e) =>
                save({ ...settings, autoArrange: e.target.checked, board: e.target.checked ? pack(tiles) : tiles })
              }
            />
          </label>
          <button
            className={confirmReset ? "on" : ""}
            onBlur={() => setConfirmReset(false)}
            onClick={() => {
              if (!confirmReset) return setConfirmReset(true);
              setConfirmReset(false);
              save({ ...settings, board: null, autoArrange: true }); // back to the default board
            }}
          >
            {confirmReset ? "Reset board?" : "Reset"}
          </button>
          <button className="add-tiles" onClick={() => setAdding(true)}>
            Add tiles
          </button>
        </div>
      )}

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
      ) : tiles.length === 0 && !editing ? (
        <div className="welcome">
          <p className="hint">Your home is empty.</p>
          <button onClick={() => (setEditing(true), setAdding(true))}>Add tiles</button>
        </div>
      ) : (
        <Board
          tiles={tiles}
          open={(id) => setView(modules[id].category)}
          editing={editing}
          auto={auto}
          onChange={setBoard}
          onLongPress={() => setEditing(true)}
        />
      )}
      {editHome && adding && (
        <AddTiles
          enabled={enabled}
          tiles={tiles}
          onAdd={(id, size: Size) => {
            const [w, h] = size.split("x").map(Number);
            setBoard(add(tiles, id, w, h, auto));
            setAdding(false);
          }}
          onClose={() => setAdding(false)}
          openModules={() => setView("modules")}
        />
      )}
      {view === "home" && error && <p className="error">{error}</p>}
    </main>
    </SettingsContext.Provider>
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

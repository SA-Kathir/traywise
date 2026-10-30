import type { ModuleId, Settings } from "../bindings";
import { categories, modules, type Category } from "../modules";
import { withModules } from "./HomeBoard";
import { useSave } from "./Settings";

const MODULE_IDS = Object.keys(modules) as ModuleId[];

/** Switch modules on and off, grouped by category, each with what it does and costs while idle. */
export function ModulesPage(props: { settings: Settings; onChange: (next: Settings) => void }) {
  const { settings } = props;
  const { save, error } = useSave(props);
  const on = settings.enabledModules;
  const allOn = MODULE_IDS.every((m) => on.includes(m));

  /** Saves which modules are on; the home board loses the tiles of those switched off and gets the
   *  default tiles of those switched on. */
  function saveOn(enabledModules: ModuleId[]) {
    const board = withModules(settings.board, on, enabledModules, settings.autoArrange);
    save({ ...settings, enabledModules, board });
  }

  function toggle(id: ModuleId, enable: boolean) {
    // Keep the registry's order, so the tabs don't reshuffle.
    saveOn(MODULE_IDS.filter((m) => (m === id ? enable : on.includes(m))));
  }

  return (
    <>
      <section>
        <label className="setting">
          <span className="setting-text">
            <span className="setting-label">All modules</span>
            <span className="setting-desc">
              {on.length} of {MODULE_IDS.length} on
            </span>
          </span>
          <input
            type="checkbox"
            aria-label="All modules"
            checked={allOn}
            // The knob waits in the middle when only some are on.
            ref={(box) => {
              if (box) box.indeterminate = !allOn && on.length > 0;
            }}
            onChange={(e) => saveOn(e.target.checked ? MODULE_IDS : [])}
          />
        </label>
      </section>
      {(Object.keys(categories) as Category[]).map((c) => (
        <section key={c}>
          <h2>{categories[c].name}</h2>
          {MODULE_IDS.filter((id) => modules[id].category === c).map((id) => (
            <label key={id} className="setting module">
              <span className="module-icon" aria-hidden>
                {modules[id].icon}
              </span>
              <span className="setting-text">
                <span className="setting-label">{modules[id].name}</span>
                <span className="setting-desc">{modules[id].description}</span>
                <span className="module-idle">Idle: {modules[id].idle}</span>
              </span>
              <input type="checkbox" checked={on.includes(id)} onChange={(e) => toggle(id, e.target.checked)} />
            </label>
          ))}
        </section>
      ))}
      {error && <p className="error">{error}</p>}
    </>
  );
}

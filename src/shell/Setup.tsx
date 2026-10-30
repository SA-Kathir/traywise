import { useState } from "react";
import type { Look, Settings } from "../bindings";
import { ModulesPage } from "./ModulesPage";
import { useSave } from "./Settings";

const LOOKS: { id: Look; label: string; detail: string }[] = [
  { id: "dot", label: "Dot", detail: "Monochrome tiles over a dot grid, with one accent colour. Follows light and dark mode." },
  { id: "liquid", label: "Liquid", detail: "Glass cards over a soft colour you pick." },
  { id: "windows", label: "Windows", detail: "Flat, like Windows' own flyouts. Follows light and dark mode." },
];

/** First run: pick a look (it applies at once, so the panel itself is the preview), then modules. */
export function Setup(props: { settings: Settings; onChange: (next: Settings) => void }) {
  const { settings } = props;
  const { save, error } = useSave(props);
  const [step, setStep] = useState<"look" | "modules">("look");

  if (step === "modules") {
    return (
      <>
        {/* Done sits up top: the list is taller than the panel. */}
        <header className="row">
          <h1 className="grow">Pick your modules</h1>
          <button className="primary" onClick={() => save({ ...settings, onboarded: true })}>
            Done
          </button>
        </header>
        <ModulesPage {...props} />
        {error && <p className="error">{error}</p>}
      </>
    );
  }
  return (
    <>
      <div className="setup-intro">
        <p className="welcome-name">Traywise</p>
        <p className="hint">Pick a look. You can change it later in Settings.</p>
      </div>
      <div className="looks" role="radiogroup" aria-label="Look">
        {LOOKS.map((l) => (
          <button
            key={l.id}
            role="radio"
            aria-checked={settings.look === l.id}
            className={settings.look === l.id ? "on" : ""}
            onClick={() => save({ ...settings, look: l.id })}
          >
            <span className="tile-label">{l.label}</span>
            <span className="tile-detail">{l.detail}</span>
          </button>
        ))}
      </div>
      <button className="primary" onClick={() => setStep("modules")}>
        Next
      </button>
      {error && <p className="error">{error}</p>}
    </>
  );
}

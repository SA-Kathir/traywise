//! The module contract. A module is one feature (like the volume mixer) that can be switched on
//! or off. A disabled module is never started: no threads, hooks or subscriptions.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::modules::audio_devices::AudioDevices;
use crate::modules::keep_awake::KeepAwake;
use crate::modules::system_monitor::SystemMonitor;
use crate::modules::volume_mixer::VolumeMixer;
use crate::settings::Settings;

/// Every built-in module. The UI has a matching entry for each one in `src/modules/index.ts`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum ModuleId {
    VolumeMixer,
    KeepAwake,
    AudioDevices,
    SystemMonitor,
}

pub trait Module: Send {
    fn id(&self) -> ModuleId;
    /// Starts the module's work. Only called when the module is switched on.
    fn start(&mut self) -> Result<(), String>;
    /// Stops it and releases everything `start` took.
    fn stop(&mut self);
    /// Called after `start`, and whenever settings change while the module runs.
    fn apply_settings(&mut self, _settings: &Settings) {}
    /// Called while running, whenever the panel opens or closes.
    fn panel_visible(&mut self, _visible: bool) {}
}

/// All built-in modules, created but not started. Creating one must be cheap and do nothing.
fn registry(app: &AppHandle) -> Vec<Box<dyn Module>> {
    vec![
        Box::new(VolumeMixer::new(app.clone())),
        Box::new(KeepAwake::new(app.clone())),
        Box::new(AudioDevices::new(app.clone())),
        Box::new(SystemMonitor::new(app.clone())),
    ]
}

struct Slot {
    module: Box<dyn Module>,
    running: bool,
}

pub struct Modules(Mutex<Vec<Slot>>);

impl Modules {
    pub fn new(app: &AppHandle) -> Self {
        Self::with(registry(app))
    }

    fn with(modules: Vec<Box<dyn Module>>) -> Self {
        Self(Mutex::new(modules.into_iter().map(|module| Slot { module, running: false }).collect()))
    }

    /// Starts the enabled modules that aren't running, and stops the running ones that were disabled.
    pub fn apply(&self, settings: &Settings) {
        for slot in self.0.lock().unwrap().iter_mut() {
            let wanted = settings.enabled_modules.contains(&slot.module.id());
            if wanted && !slot.running {
                match slot.module.start() {
                    Ok(()) => slot.running = true,
                    Err(e) => eprintln!("module {:?} failed to start: {e}", slot.module.id()),
                }
            } else if !wanted && slot.running {
                slot.module.stop();
                slot.running = false;
            }
            if slot.running {
                slot.module.apply_settings(settings);
            }
        }
    }

    pub fn panel_visible(&self, visible: bool) {
        for slot in self.0.lock().unwrap().iter_mut().filter(|s| s.running) {
            slot.module.panel_visible(visible);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct Fake(Arc<Mutex<Vec<&'static str>>>);
    impl Module for Fake {
        fn id(&self) -> ModuleId {
            ModuleId::VolumeMixer
        }
        fn start(&mut self) -> Result<(), String> {
            self.0.lock().unwrap().push("start");
            Ok(())
        }
        fn stop(&mut self) {
            self.0.lock().unwrap().push("stop");
        }
        fn apply_settings(&mut self, _: &Settings) {
            self.0.lock().unwrap().push("settings");
        }
    }

    #[test]
    fn starts_and_stops_only_on_change() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let modules = Modules::with(vec![Box::new(Fake(calls.clone()))]);
        let enabled = |enabled_modules| Settings { enabled_modules, ..Settings::default() };
        modules.apply(&enabled(vec![])); // disabled from the start: never started
        modules.apply(&enabled(vec![ModuleId::VolumeMixer]));
        modules.apply(&enabled(vec![ModuleId::VolumeMixer])); // already running: no second start
        modules.apply(&enabled(vec![]));
        modules.apply(&enabled(vec![])); // already stopped: no second stop
        assert_eq!(*calls.lock().unwrap(), ["start", "settings", "settings", "stop"]);
    }
}

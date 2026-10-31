//! Clock: the time and date, and optionally a second time zone, on the home board.
//!
//! Nothing runs in Rust. The UI reads the time itself and only updates it, once a minute, while the
//! panel is open. This module exists so the clock can be switched on or off like the others.

use tauri::AppHandle;

use crate::module::{Module, ModuleId};

pub struct Clock;

impl Clock {
    pub fn new(_app: AppHandle) -> Self {
        Self
    }
}

impl Module for Clock {
    fn id(&self) -> ModuleId {
        ModuleId::Clock
    }

    fn start(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn stop(&mut self) {}
}

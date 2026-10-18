//! Volume mixer: per-app volume and mute. A stub until milestone 1, step 6: it only reports
//! when it's started and stopped, which shows that a disabled module never starts.

use crate::module::{Module, ModuleId};

pub struct VolumeMixer;

impl Module for VolumeMixer {
    fn id(&self) -> ModuleId {
        ModuleId::VolumeMixer
    }

    fn start(&mut self) -> Result<(), String> {
        println!("volume-mixer started");
        Ok(())
    }

    fn stop(&mut self) {
        println!("volume-mixer stopped");
    }
}

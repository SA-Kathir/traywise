mod module;
mod modules;
mod settings;
mod shell;

use tauri::Manager;
use tauri_specta::{collect_commands, collect_events};

/// Everything the UI can call or listen to. `src/bindings.ts` is generated from this list.
fn bindings() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::new()
        .commands(collect_commands![
            shell::panel::panel_painted,
            settings::get_settings,
            settings::set_settings,
            settings::get_autostart,
            settings::set_autostart,
        ])
        .events(collect_events![shell::panel::PanelShown])
}

/// Rewrites `src/bindings.ts` if it no longer matches the Rust side. Returns whether it changed.
/// Debug builds only: it writes into the source tree.
#[cfg(debug_assertions)]
fn export_bindings(bindings: &tauri_specta::Builder<tauri::Wry>) -> bool {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/bindings.ts");
    let fresh_path = std::env::temp_dir().join(format!("traywise-bindings-{}.ts", std::process::id()));
    bindings
        .export(specta_typescript::Typescript::default(), &fresh_path)
        .expect("failed to export TypeScript bindings");
    let fresh = std::fs::read_to_string(&fresh_path).expect("failed to read exported bindings");
    let _ = std::fs::remove_file(&fresh_path);
    // Git on Windows may have checked the file out with CRLF line endings; that isn't staleness.
    let current = std::fs::read_to_string(path).unwrap_or_default().replace("\r\n", "\n");
    if current == fresh {
        return false;
    }
    std::fs::write(path, fresh).expect("failed to write src/bindings.ts");
    true
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let bindings = bindings();
    #[cfg(debug_assertions)]
    export_bindings(&bindings);

    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .invoke_handler(bindings.invoke_handler())
        .setup(move |app| {
            bindings.mount_events(app);
            // Settings and modules first: the panel's UI asks for settings as soon as it loads.
            let settings = settings::SettingsState::load(app.handle())?;
            let modules = module::Modules::new();
            modules.apply(&settings.get().enabled_modules);
            app.manage(settings);
            app.manage(modules);
            shell::panel::create(app)?;
            shell::tray::create(app)?;
            #[cfg(feature = "autotest")]
            shell::panel::autotest(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    /// Fails (and regenerates the file) when someone changed a command or event without
    /// updating `src/bindings.ts`. Run `cargo test` again after committing the new file.
    #[test]
    fn bindings_are_up_to_date() {
        assert!(
            !super::export_bindings(&super::bindings()),
            "src/bindings.ts was stale; it has been regenerated, review and commit it"
        );
    }
}

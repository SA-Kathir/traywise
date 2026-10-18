fn main() {
    // Tauri embeds its Windows app manifest (which asks for the modern Common Controls DLL)
    // into the app .exe only, so `cargo test` binaries crash on start with
    // STATUS_ENTRYPOINT_NOT_FOUND. Embed the same manifest into every binary instead.
    let manifest = std::path::absolute("windows-app-manifest.xml").expect("manifest path");
    println!("cargo:rerun-if-changed=windows-app-manifest.xml");
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());

    let windows = tauri_build::WindowsAttributes::new_without_app_manifest();
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
        .expect("failed to run tauri-build");
}

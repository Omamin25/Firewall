use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    link_npcap();
    stage_windivert_runtime();
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .windows_attributes(
                tauri_build::WindowsAttributes::new().app_manifest(include_str!("app.manifest")),
            )
            .app_manifest(tauri_build::AppManifest::new().commands(&[
                "demo_health_check",
                "database_smoke_test",
                "list_capture_interfaces",
                "start_capture",
                "stop_capture",
                "get_packet_detail",
                "subscribe_packet_stream",
                "list_flows",
                "get_flow_history_summary",
                "list_rules",
                "create_rule",
                "update_rule",
                "toggle_rule",
                "delete_rule",
                "draft_rule_from_flow",
                "draft_rule_from_packet",
                "create_rule_from_flow",
                "get_enforcement_status",
                "update_retention_policy",
                "prune_flow_history",
                "get_system_health",
            ])),
    )?;
    Ok(())
}

/// The `pcap` crate links against `wpcap.lib`, which ships only in the Npcap *SDK* —
/// the Npcap runtime installer does not provide it. `vendor/npcap/lib/wpcap.lib` is an
/// import library generated from the installed `wpcap.dll` (see
/// `scripts/generate-wpcap-lib.ps1`), so the build works without the SDK. An SDK on
/// `LIB` still wins because the linker searches `LIB` before these extra paths.
fn link_npcap() {
    if !cfg!(target_os = "windows") {
        return;
    }
    let vendored = vendor_dir().join("npcap").join("lib");
    if vendored.join("wpcap.lib").exists() {
        println!("cargo:rustc-link-search=native={}", vendored.display());
    }
    println!("cargo:rerun-if-changed=../vendor/npcap/lib/wpcap.lib");
}

/// `WinDivert.dll` is a static import of `windivert-sys`, so *every* binary this crate
/// produces needs it beside the executable — and the DLL in turn loads its driver from
/// its own directory. Copying both into the profile's output directories keeps
/// `cargo test`, `cargo run --example`, and `tauri dev` working without a system-wide
/// install (packaging is Phase 6's job).
fn stage_windivert_runtime() {
    if !cfg!(target_os = "windows") {
        return;
    }
    let source = vendor_dir()
        .join("windivert")
        .join("WinDivert-2.2.2-A")
        .join("x64");
    println!("cargo:rerun-if-changed={}", source.display());
    let Some(profile_dir) = profile_dir() else {
        println!(
            "cargo:warning=could not locate the target directory; WinDivert.dll was not staged"
        );
        return;
    };
    // Test and example binaries live in their own subdirectories and each gets its own
    // DLL search path, so all three need a copy.
    for directory in [
        profile_dir.clone(),
        profile_dir.join("deps"),
        profile_dir.join("examples"),
    ] {
        for file in ["WinDivert.dll", "WinDivert64.sys"] {
            copy_if_newer(&source.join(file), &directory.join(file));
        }
    }
}

fn copy_if_newer(source: &Path, destination: &Path) {
    let Some(parent) = destination.parent() else {
        return;
    };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }
    // A running debug build holds a lock on the DLL it loaded, so a failed copy of an
    // already-present file is expected and must not fail the build.
    if destination.exists() && !is_newer(source, destination) {
        return;
    }
    if let Err(error) = std::fs::copy(source, destination) {
        println!(
            "cargo:warning=could not stage {}: {error}",
            destination.display()
        );
    }
}

fn is_newer(source: &Path, destination: &Path) -> bool {
    let modified = |path: &Path| {
        std::fs::metadata(path)
            .and_then(|meta| meta.modified())
            .ok()
    };
    match (modified(source), modified(destination)) {
        (Some(source), Some(destination)) => source > destination,
        _ => true,
    }
}

fn vendor_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("vendor")
}

/// `OUT_DIR` is `<target>/<profile>/build/<pkg>-<hash>/out`, so the profile directory is
/// three levels up. Derived rather than assumed so a custom `CARGO_TARGET_DIR` works.
fn profile_dir() -> Option<PathBuf> {
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR")?);
    Some(out_dir.ancestors().nth(3)?.to_path_buf())
}

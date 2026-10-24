#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = cognton_lib::run() {
        tracing::error!(%error, "Cognton exited unexpectedly");
    }
}

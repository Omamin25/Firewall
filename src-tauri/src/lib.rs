// Modules are public so the integration tests in `tests/` can exercise the parsers
// against byte fixtures (`code_standards.md` §5).
pub mod attribution;
pub mod capture;
pub mod enforcement;
pub mod flow;
pub mod ipc;
pub mod model;
pub mod parse;
pub mod rules;
pub mod store;

use std::{fs, path::PathBuf, sync::Arc};

use anyhow::Context;
use ipc::AppState;
use tauri::Manager;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn run() -> anyhow::Result<()> {
    init_logging()?;
    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .context("resolve app data directory")?;
            let state = match store::Store::open(&app_data_dir) {
                Ok(store) => AppState::new(Arc::new(store), None),
                Err(error) => {
                    tracing::error!(%error, "database initialization failed");
                    let fallback_dir = std::env::temp_dir().join("cognton-fallback");
                    let store =
                        store::Store::open(&fallback_dir).context("open fallback database")?;
                    AppState::new(
                        Arc::new(store),
                        Some(format!("Local database could not open: {error}")),
                    )
                }
            };
            app.manage(state);
            // Enforcement is independent of capture: rules apply from launch, and the
            // SOCKET-layer listener sharpens attribution whether or not any rule exists.
            if let Some(state) = app.try_state::<AppState>() {
                state.start_enforcement();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ipc::demo_health_check,
            ipc::database_smoke_test,
            ipc::list_capture_interfaces,
            ipc::start_capture,
            ipc::stop_capture,
            ipc::get_packet_detail,
            ipc::subscribe_packet_stream,
            ipc::flows::list_flows,
            ipc::flows::get_flow_history_summary,
            ipc::rules::list_rules,
            ipc::rules::create_rule,
            ipc::rules::update_rule,
            ipc::rules::toggle_rule,
            ipc::rules::delete_rule,
            ipc::rules::draft_rule_from_flow,
            ipc::rules::draft_rule_from_packet,
            ipc::rules::create_rule_from_flow,
            ipc::rules::get_enforcement_status,
            ipc::flows::update_retention_policy,
            ipc::flows::prune_flow_history,
            ipc::flows::get_system_health
        ])
        .run(tauri::generate_context!())
        .context("run Tauri application")
}

fn init_logging() -> anyhow::Result<()> {
    let log_dir = log_directory()?;
    fs::create_dir_all(&log_dir).context("create log directory")?;
    let file_appender = tracing_appender::rolling::daily(log_dir, "cognton.log");
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(file_appender)
                .with_ansi(false),
        )
        .try_init()
        .context("initialize tracing")?;
    Ok(())
}

pub fn log_directory() -> anyhow::Result<PathBuf> {
    dirs::data_local_dir()
        .map(|path| path.join("Cognton").join("logs"))
        .context("resolve local application data directory")
}

#[cfg(windows)]
fn is_elevated() -> bool {
    // SAFETY: IsUserAnAdmin has no input pointers and returns a Win32 BOOL.
    unsafe { windows::Win32::UI::Shell::IsUserAnAdmin().as_bool() }
}

#[cfg(not(windows))]
fn is_elevated() -> bool {
    false
}

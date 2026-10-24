//! Flow-facing commands: the initial Connections snapshot and the history summary.
//! Live updates arrive as batched diffs on the stream channel, not through these.

use tauri::State;

use super::{
    dto::{FlowDto, FlowHistorySummaryDto, PruneResultDto, SystemHealthDto},
    AppState,
};
use crate::is_elevated;

/// Every live flow, newest activity first. The UI calls this once when the view mounts
/// (and after a window reload) because the stream only carries diffs.
#[tauri::command]
pub fn list_flows(state: State<'_, AppState>) -> Vec<FlowDto> {
    state
        .tracker
        .snapshot()
        .iter()
        .map(FlowDto::from_flow)
        .collect()
}

/// Size and span of the persisted `flows` table plus the retention policy in force, so
/// pruning is observable without shipping history over IPC.
#[tauri::command]
pub fn get_flow_history_summary(
    state: State<'_, AppState>,
) -> Result<FlowHistorySummaryDto, String> {
    let summary = state
        .store
        .flow_history_summary()
        .map_err(|error| error.to_string())?;
    let policy = state.store.retention_policy();
    Ok(FlowHistorySummaryDto {
        stored_flows: summary.stored_flows,
        oldest_seen: summary.oldest_seen,
        newest_seen: summary.newest_seen,
        retention_hours: policy.hours,
        max_rows: policy.max_rows,
    })
}

/// Accepted retention window. One hour is the shortest useful rolling window; 7200h
/// (300 days) is the longest, so a typo cannot turn retention off or make it unbounded.
const RETENTION_HOURS_RANGE: std::ops::RangeInclusive<u32> = 1..=7200;
/// Accepted row cap. The floor keeps history meaningful; the ceiling bounds the database
/// regardless of what the webview asks for.
const MAX_ROWS_RANGE: std::ops::RangeInclusive<u64> = 100..=1_000_000;

/// Validates a requested retention policy. Split out from the command so the limits the
/// UI mirrors in `settings-page.tsx` can be unit-tested (`code_standards.md` §5).
fn validate_retention(retention_hours: u32, max_rows: u64) -> Result<(), String> {
    if !RETENTION_HOURS_RANGE.contains(&retention_hours) {
        return Err(format!(
            "Retention hours must be between {} and {}",
            RETENTION_HOURS_RANGE.start(),
            RETENTION_HOURS_RANGE.end()
        ));
    }
    if !MAX_ROWS_RANGE.contains(&max_rows) {
        return Err(format!(
            "Max rows cap must be between {} and {}",
            MAX_ROWS_RANGE.start(),
            MAX_ROWS_RANGE.end()
        ));
    }
    Ok(())
}

/// Updates the retention policy parameters (`flow_retention_hours` and `flow_max_rows`) in settings.
#[tauri::command]
pub fn update_retention_policy(
    state: State<'_, AppState>,
    retention_hours: u32,
    max_rows: u64,
) -> Result<FlowHistorySummaryDto, String> {
    validate_retention(retention_hours, max_rows)?;
    state
        .store
        .set_setting("flow_retention_hours", &retention_hours.to_string())
        .map_err(|error| error.to_string())?;
    state
        .store
        .set_setting("flow_max_rows", &max_rows.to_string())
        .map_err(|error| error.to_string())?;

    get_flow_history_summary(state)
}

/// Manually triggers flow history retention pruning.
#[tauri::command]
pub fn prune_flow_history(state: State<'_, AppState>) -> Result<PruneResultDto, String> {
    let policy = state.store.retention_policy();
    let removed_count = state
        .store
        .prune_flows(policy, chrono::Utc::now())
        .map_err(|error| error.to_string())?;
    let summary = get_flow_history_summary(state)?;
    Ok(PruneResultDto {
        removed_count,
        summary,
    })
}

/// System diagnostic and resilience health status.
#[tauri::command]
pub fn get_system_health(state: State<'_, AppState>) -> Result<SystemHealthDto, String> {
    let interfaces_res = state.capture.list_interfaces();
    let npcap_available = interfaces_res.is_ok();
    let active_interfaces_count = state.capture.active_interfaces().len();
    let active_flows_count = state.tracker.active_count();
    let active_rules_count = state.rules.len();

    let enforcement_status = state.enforcement.status().into();
    let log_dir = crate::log_directory()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "Unknown".to_owned());

    Ok(SystemHealthDto {
        is_elevated: is_elevated(),
        database_ready: state.startup_error.is_none(),
        database_path: state.store.path().to_string_lossy().into_owned(),
        log_directory: log_dir,
        npcap_available,
        active_interfaces_count,
        active_flows_count,
        active_rules_count,
        enforcement_status,
    })
}

#[cfg(test)]
mod tests {
    use super::{validate_retention, MAX_ROWS_RANGE, RETENTION_HOURS_RANGE};
    use crate::store::{DEFAULT_MAX_ROWS, DEFAULT_RETENTION_HOURS};

    #[test]
    fn accepts_the_defaults_and_both_bounds() {
        validate_retention(DEFAULT_RETENTION_HOURS, DEFAULT_MAX_ROWS)
            .expect("the shipped defaults must be valid");
        validate_retention(*RETENTION_HOURS_RANGE.start(), *MAX_ROWS_RANGE.start())
            .expect("the lower bounds are inclusive");
        validate_retention(*RETENTION_HOURS_RANGE.end(), *MAX_ROWS_RANGE.end())
            .expect("the upper bounds are inclusive");
    }

    #[test]
    fn rejects_a_zero_or_oversized_retention_window() {
        // Zero would silently disable retention, deleting history on the next sweep.
        assert!(validate_retention(0, DEFAULT_MAX_ROWS).is_err());
        assert!(validate_retention(RETENTION_HOURS_RANGE.end() + 1, DEFAULT_MAX_ROWS).is_err());
    }

    #[test]
    fn rejects_a_row_cap_outside_the_accepted_range() {
        assert!(validate_retention(DEFAULT_RETENTION_HOURS, 0).is_err());
        assert!(validate_retention(DEFAULT_RETENTION_HOURS, 99).is_err());
        assert!(validate_retention(DEFAULT_RETENTION_HOURS, MAX_ROWS_RANGE.end() + 1).is_err());
    }

    #[test]
    fn the_error_message_names_the_offending_field() {
        let hours = validate_retention(0, DEFAULT_MAX_ROWS).expect_err("should reject");
        assert!(hours.contains("Retention hours"), "got: {hours}");
        let rows = validate_retention(DEFAULT_RETENTION_HOURS, 1).expect_err("should reject");
        assert!(rows.contains("Max rows"), "got: {rows}");
    }
}

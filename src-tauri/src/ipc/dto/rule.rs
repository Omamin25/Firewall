use serde::{Deserialize, Serialize};

use crate::{
    enforcement::EnforcementStatus,
    model::{Rule, RuleAction, RuleDirection, RuleDraft, RuleProtocol},
    rules::IpMatcher,
};

/// Longest accepted rule name. Generous for a description, short enough that the table
/// stays readable.
const MAX_NAME_LEN: usize = 120;
/// `MAX_PATH` is 260, but long paths reach 32k; 512 covers real executables without
/// letting the webview push an unbounded string into the database.
const MAX_PROCESS_PATH_LEN: usize = 512;
/// A string SID is at most 184 characters.
const MAX_SID_LEN: usize = 184;
const MAX_REMOTE_IP_LEN: usize = 64;
/// Priorities are a small ordering space, not an arbitrary integer.
const MAX_PRIORITY: i64 = 100_000;

/// Row shape for the Rules manager. Mirrored by `Rule` in `src/types/ipc.ts`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleDto {
    pub id: i64,
    pub name: String,
    pub action: String,
    pub direction: String,
    pub protocol: String,
    pub remote_ip: Option<String>,
    pub remote_port: Option<u16>,
    pub local_port: Option<u16>,
    pub process_path: Option<String>,
    pub user_sid: Option<String>,
    pub enabled: bool,
    pub priority: i64,
    pub hit_count: u64,
    pub created_at: String,
    pub updated_at: String,
}

impl RuleDto {
    /// `hit_count` is overridden with the live in-memory total when the rule is loaded in
    /// the hot set, so the UI never shows a counter behind what the user just saw.
    pub fn from_rule(rule: &Rule, live_hits: Option<u64>) -> Self {
        Self {
            id: rule.id,
            name: rule.draft.name.clone(),
            action: rule.draft.action.as_str().to_owned(),
            direction: rule.draft.direction.as_str().to_owned(),
            protocol: rule.draft.protocol.as_str().to_owned(),
            remote_ip: rule.draft.remote_ip.clone(),
            remote_port: rule.draft.remote_port,
            local_port: rule.draft.local_port,
            process_path: rule.draft.process_path.clone(),
            user_sid: rule.draft.user_sid.clone(),
            enabled: rule.draft.enabled,
            priority: rule.draft.priority,
            hit_count: live_hits.unwrap_or(rule.hit_count),
            created_at: rule.created_at.to_rfc3339(),
            updated_at: rule.updated_at.to_rfc3339(),
        }
    }
}

/// Rule criteria as submitted by the webview. Everything here is untrusted
/// (`architecture.md` §5 rule 1), so [`RuleDraftDto::validate`] is the only way to turn it
/// into a [`RuleDraft`].
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleDraftDto {
    pub name: String,
    pub action: String,
    pub direction: String,
    pub protocol: String,
    pub remote_ip: Option<String>,
    pub remote_port: Option<u16>,
    pub local_port: Option<u16>,
    pub process_path: Option<String>,
    pub user_sid: Option<String>,
    pub enabled: bool,
    pub priority: i64,
}

impl RuleDraftDto {
    pub fn validate(self) -> Result<RuleDraft, String> {
        let name = text(&self.name, "name", MAX_NAME_LEN)?
            .ok_or_else(|| "a rule needs a name".to_owned())?;
        let action = RuleAction::parse(&self.action).map_err(|error| error.to_string())?;
        let direction = RuleDirection::parse(&self.direction).map_err(|error| error.to_string())?;
        let protocol = RuleProtocol::parse(&self.protocol).map_err(|error| error.to_string())?;

        let remote_ip = match text(
            &self.remote_ip.unwrap_or_default(),
            "remote address",
            MAX_REMOTE_IP_LEN,
        )? {
            Some(value) => {
                // Parsed here as well as at load time so a bad block is rejected while the
                // user is still looking at the form.
                IpMatcher::parse(&value).map_err(|error| error.to_string())?;
                Some(value)
            }
            None => None,
        };
        let process_path = text(
            &self.process_path.unwrap_or_default(),
            "process path",
            MAX_PROCESS_PATH_LEN,
        )?;
        let user_sid = match text(&self.user_sid.unwrap_or_default(), "user SID", MAX_SID_LEN)? {
            Some(value) => Some(validated_sid(value)?),
            None => None,
        };

        if !(0..=MAX_PRIORITY).contains(&self.priority) {
            return Err(format!("priority must be between 0 and {MAX_PRIORITY}"));
        }
        for (port, label) in [
            (self.remote_port, "remote port"),
            (self.local_port, "local port"),
        ] {
            if port == Some(0) {
                return Err(format!("{label} must be between 1 and 65535"));
            }
        }

        Ok(RuleDraft {
            name,
            action,
            direction,
            protocol,
            remote_ip,
            remote_port: self.remote_port,
            local_port: self.local_port,
            process_path,
            user_sid,
            enabled: self.enabled,
            priority: self.priority,
        })
    }
}

/// Trims, rejects control characters, and treats an empty value as "unset".
fn text(value: &str, field: &str, max_len: usize) -> Result<Option<String>, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > max_len {
        return Err(format!("{field} must be at most {max_len} characters"));
    }
    if value.chars().any(char::is_control) {
        return Err(format!("{field} contains unsupported characters"));
    }
    Ok(Some(value.to_owned()))
}

/// String SIDs are `S-1-<authority>-<sub authorities>`; anything else would never match a
/// resolved token and is more likely a typo than an intent.
fn validated_sid(value: String) -> Result<String, String> {
    let looks_like_sid = value.len() > 4
        && value[..4].eq_ignore_ascii_case("S-1-")
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-');
    if looks_like_sid {
        Ok(value)
    } else {
        Err("a user SID looks like 'S-1-5-21-…'".to_owned())
    }
}

/// State of the enforcement engine, for the engine-status pill and the Rules view.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnforcementStatusDto {
    pub available: bool,
    pub enforcing: bool,
    pub observing: bool,
    pub enabled_rules: usize,
    pub block_rules: usize,
    pub blocked_packets: u64,
    pub socket_events: u64,
    pub tracked_endpoints: usize,
    pub error: Option<String>,
}

impl From<EnforcementStatus> for EnforcementStatusDto {
    fn from(status: EnforcementStatus) -> Self {
        Self {
            available: status.available,
            enforcing: status.enforcing,
            observing: status.observing,
            enabled_rules: status.enabled_rules,
            block_rules: status.block_rules,
            blocked_packets: status.blocked_packets,
            socket_events: status.socket_events,
            tracked_endpoints: status.tracked_endpoints,
            error: status.error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft() -> RuleDraftDto {
        RuleDraftDto {
            name: "  block example  ".to_owned(),
            action: "block".to_owned(),
            direction: "out".to_owned(),
            protocol: "tcp".to_owned(),
            remote_ip: Some(" 93.184.216.0/24 ".to_owned()),
            remote_port: Some(443),
            local_port: None,
            process_path: Some("telemetry.exe".to_owned()),
            user_sid: Some("S-1-5-21-7-1001".to_owned()),
            enabled: true,
            priority: 50,
        }
    }

    #[test]
    fn trims_and_accepts_a_well_formed_draft() {
        let validated = draft().validate().expect("draft should validate");
        assert_eq!(validated.name, "block example");
        assert_eq!(validated.remote_ip.as_deref(), Some("93.184.216.0/24"));
        assert_eq!(validated.action, RuleAction::Block);
    }

    #[test]
    fn treats_blank_criteria_as_unset() {
        let mut input = draft();
        input.remote_ip = Some("   ".to_owned());
        input.process_path = Some(String::new());
        input.user_sid = None;
        let validated = input.validate().expect("draft should validate");
        assert!(validated.remote_ip.is_none());
        assert!(validated.process_path.is_none());
        assert!(validated.user_sid.is_none());
    }

    #[test]
    fn rejects_a_missing_name() {
        let mut input = draft();
        input.name = "   ".to_owned();
        assert!(input.validate().is_err());
    }

    #[test]
    fn rejects_unknown_enum_values() {
        for mutate in [
            |input: &mut RuleDraftDto| input.action = "quarantine".to_owned(),
            |input: &mut RuleDraftDto| input.direction = "sideways".to_owned(),
            |input: &mut RuleDraftDto| input.protocol = "sctp".to_owned(),
        ] {
            let mut input = draft();
            mutate(&mut input);
            assert!(input.validate().is_err());
        }
    }

    #[test]
    fn rejects_a_malformed_remote_address() {
        let mut input = draft();
        input.remote_ip = Some("93.184.216.0/99".to_owned());
        assert!(input.validate().is_err());
    }

    #[test]
    fn rejects_control_characters_and_oversized_values() {
        let mut input = draft();
        input.process_path = Some("tele\u{0}metry.exe".to_owned());
        assert!(input.validate().is_err());

        let mut input = draft();
        input.name = "x".repeat(MAX_NAME_LEN + 1);
        assert!(input.validate().is_err());
    }

    #[test]
    fn rejects_out_of_range_ports_and_priorities() {
        let mut input = draft();
        input.remote_port = Some(0);
        assert!(input.validate().is_err());

        let mut input = draft();
        input.priority = -1;
        assert!(input.validate().is_err());

        let mut input = draft();
        input.priority = MAX_PRIORITY + 1;
        assert!(input.validate().is_err());
    }

    #[test]
    fn rejects_values_that_are_not_sids() {
        let mut input = draft();
        input.user_sid = Some(r"OMEN\vraj".to_owned());
        assert!(input.validate().is_err());
    }
}

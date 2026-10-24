//! Firewall rule domain types. Wire shapes live in `ipc/dto/rule.rs`; these are the
//! validated values the store and the matching engine work with.

use chrono::{DateTime, Utc};
use thiserror::Error;

use super::{Direction, TransportProtocol};

#[derive(Debug, Error)]
#[error("'{value}' is not a valid rule {field}")]
pub struct RuleFieldError {
    pub field: &'static str,
    pub value: String,
}

impl RuleFieldError {
    fn new(field: &'static str, value: &str) -> Self {
        Self {
            field,
            value: value.to_owned(),
        }
    }
}

/// Block wins over allow only when it sorts first; see [`crate::rules::RuleSet`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RuleAction {
    Allow,
    Block,
}

impl RuleAction {
    pub fn as_str(self) -> &'static str {
        match self {
            RuleAction::Allow => "allow",
            RuleAction::Block => "block",
        }
    }

    pub fn parse(value: &str) -> Result<Self, RuleFieldError> {
        match value {
            "allow" => Ok(RuleAction::Allow),
            "block" => Ok(RuleAction::Block),
            other => Err(RuleFieldError::new("action", other)),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RuleDirection {
    In,
    Out,
    Both,
}

impl RuleDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            RuleDirection::In => "in",
            RuleDirection::Out => "out",
            RuleDirection::Both => "both",
        }
    }

    pub fn parse(value: &str) -> Result<Self, RuleFieldError> {
        match value {
            "in" => Ok(RuleDirection::In),
            "out" => Ok(RuleDirection::Out),
            "both" => Ok(RuleDirection::Both),
            other => Err(RuleFieldError::new("direction", other)),
        }
    }

    pub fn matches(self, direction: Direction) -> bool {
        match self {
            RuleDirection::Both => true,
            RuleDirection::In => direction == Direction::In,
            RuleDirection::Out => direction == Direction::Out,
        }
    }
}

/// The transports a rule can name. `architecture.md` §6 constrains the column to these
/// four, so IGMP is only reachable through `Any`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RuleProtocol {
    Tcp,
    Udp,
    Icmp,
    Any,
}

impl RuleProtocol {
    pub fn as_str(self) -> &'static str {
        match self {
            RuleProtocol::Tcp => "tcp",
            RuleProtocol::Udp => "udp",
            RuleProtocol::Icmp => "icmp",
            RuleProtocol::Any => "any",
        }
    }

    pub fn parse(value: &str) -> Result<Self, RuleFieldError> {
        match value {
            "tcp" => Ok(RuleProtocol::Tcp),
            "udp" => Ok(RuleProtocol::Udp),
            "icmp" => Ok(RuleProtocol::Icmp),
            "any" => Ok(RuleProtocol::Any),
            other => Err(RuleFieldError::new("protocol", other)),
        }
    }

    pub fn matches(self, protocol: TransportProtocol) -> bool {
        match self {
            RuleProtocol::Any => true,
            RuleProtocol::Tcp => protocol == TransportProtocol::Tcp,
            RuleProtocol::Udp => protocol == TransportProtocol::Udp,
            RuleProtocol::Icmp => protocol == TransportProtocol::Icmp,
        }
    }
}

/// The criteria a caller may set. Shared by create and update so the two paths cannot
/// drift, and validated at the IPC boundary before it reaches the store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleDraft {
    pub name: String,
    pub action: RuleAction,
    pub direction: RuleDirection,
    pub protocol: RuleProtocol,
    /// Exact address or CIDR block; `None` matches any remote.
    pub remote_ip: Option<String>,
    pub remote_port: Option<u16>,
    pub local_port: Option<u16>,
    /// Full image path, or a bare file name to match on the executable alone.
    pub process_path: Option<String>,
    pub user_sid: Option<String>,
    pub enabled: bool,
    /// Lower is evaluated first.
    pub priority: i64,
}

impl RuleDraft {
    /// A draft with no criteria at all matches every packet. That is a legitimate
    /// "block everything" rule, so it is allowed — but it is worth logging.
    pub fn matches_everything(&self) -> bool {
        self.remote_ip.is_none()
            && self.remote_port.is_none()
            && self.local_port.is_none()
            && self.process_path.is_none()
            && self.user_sid.is_none()
            && self.protocol == RuleProtocol::Any
            && self.direction == RuleDirection::Both
    }
}

/// A persisted rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    pub id: i64,
    pub draft: RuleDraft,
    pub hit_count: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// The rule that decided a packet's fate, carried on the flow it was dropped from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchedRule {
    pub id: i64,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_enum_strings() {
        for action in [RuleAction::Allow, RuleAction::Block] {
            assert_eq!(RuleAction::parse(action.as_str()).expect("valid"), action);
        }
        for direction in [RuleDirection::In, RuleDirection::Out, RuleDirection::Both] {
            assert_eq!(
                RuleDirection::parse(direction.as_str()).expect("valid"),
                direction
            );
        }
        for protocol in [
            RuleProtocol::Tcp,
            RuleProtocol::Udp,
            RuleProtocol::Icmp,
            RuleProtocol::Any,
        ] {
            assert_eq!(
                RuleProtocol::parse(protocol.as_str()).expect("valid"),
                protocol
            );
        }
    }

    #[test]
    fn rejects_unknown_enum_strings() {
        assert!(RuleAction::parse("drop").is_err());
        assert!(RuleDirection::parse("inbound").is_err());
        assert!(RuleProtocol::parse("igmp").is_err());
    }

    #[test]
    fn any_protocol_covers_igmp_but_icmp_does_not() {
        assert!(RuleProtocol::Any.matches(TransportProtocol::Igmp));
        assert!(!RuleProtocol::Icmp.matches(TransportProtocol::Igmp));
    }
}

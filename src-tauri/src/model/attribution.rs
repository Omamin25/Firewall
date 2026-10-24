/// Who owns a connection, as resolved from the OS. Every field is optional because
/// attribution is best-effort: a flow can be observed before the kernel's connection
/// table lists it, and system processes deny `OpenProcess` even to Administrators.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Attribution {
    pub pid: Option<u32>,
    pub process_name: Option<String>,
    pub process_path: Option<String>,
    /// `domain\user` for the token that owns the process.
    pub user_account: Option<String>,
    /// String SID; Phase 4 rules match on this rather than the display name.
    pub user_sid: Option<String>,
}

impl Attribution {
    /// Shared "nothing known yet" value, so the decode path can hand out a reference
    /// without allocating per frame.
    pub const EMPTY: Self = Self {
        pid: None,
        process_name: None,
        process_path: None,
        user_account: None,
        user_sid: None,
    };

    /// True once a PID was found — the point at which the resolver stops retrying.
    pub fn is_resolved(&self) -> bool {
        self.pid.is_some()
    }

    pub fn from_pid(pid: u32) -> Self {
        Self {
            pid: Some(pid),
            ..Self::default()
        }
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Globally monotonic unique identifier for events, persisting across sessions and restarts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct GlobalEventId(pub u64);

impl GlobalEventId {
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    #[inline]
    pub const fn as_u64(&self) -> u64 {
        self.0
    }

    #[inline]
    pub fn next(&self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

impl fmt::Display for GlobalEventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u64> for GlobalEventId {
    fn from(val: u64) -> Self {
        Self(val)
    }
}

/// Unique session identifier formatted as `{machine_id}_{YYYYMMDD}_{HH0000}_{uuid_short}`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Generates a standardized SessionId from machine_id, timestamp, and a short unique suffix.
    pub fn generate(machine_id: &str, wall_time: DateTime<Utc>, uuid_short: &str) -> Self {
        let date_str = wall_time.format("%Y%m%d").to_string();
        let hour_str = wall_time.format("%H0000").to_string();
        Self(format!("{machine_id}_{date_str}_{hour_str}_{uuid_short}"))
    }

    /// Verifies if the session id follows the standard 4-part naming format.
    pub fn is_valid_format(&self) -> bool {
        let parts: Vec<&str> = self.0.split('_').collect();
        if parts.len() < 4 {
            return false;
        }
        let date_part = parts[1];
        let hour_part = parts[2];
        date_part.len() == 8 && date_part.chars().all(|c| c.is_ascii_digit())
            && hour_part.len() == 6 && hour_part.chars().all(|c| c.is_ascii_digit())
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for SessionId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Sequential event ID scoped to a single hourly session partition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct SessionEventId(pub u64);

impl SessionEventId {
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    #[inline]
    pub const fn as_u64(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for SessionEventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique machine hardware/enrollment identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
pub struct MachineId(pub String);

impl MachineId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MachineId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// User username or organizational identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
pub struct UserId(pub String);

impl UserId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_event_id_ordering() {
        let id1 = GlobalEventId::new(100);
        let id2 = GlobalEventId::new(101);
        assert!(id1 < id2);
        assert_eq!(id1.next(), id2);
        assert_eq!(id1.as_u64(), 100);
    }

    #[test]
    fn test_session_id_formatting_and_parsing() {
        let now = chrono::Utc::now();
        let sid = SessionId::generate("PC001", now, "a1b2c3d4");
        assert!(sid.is_valid_format());
        assert!(sid.as_str().starts_with("PC001_"));

        let malformed = SessionId::new("invalid_session");
        assert!(!malformed.is_valid_format());
    }
}

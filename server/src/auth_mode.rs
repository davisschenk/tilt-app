//! Top-level auth mode toggle. Controls whether the server enforces OIDC
//! authentication or runs with auth disabled (dev/test instances).

use std::fmt;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AuthMode {
    Disabled,
    Oidc,
}

impl AuthMode {
    /// Parse `AUTH_MODE` from the environment. Defaults to `Disabled` if unset.
    /// Returns Err on an unrecognized value.
    pub fn from_env() -> Result<Self, String> {
        let raw = std::env::var("AUTH_MODE").unwrap_or_else(|_| "disabled".to_string());
        Self::from_str(&raw)
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "disabled" | "off" | "none" => Ok(AuthMode::Disabled),
            "oidc" => Ok(AuthMode::Oidc),
            other => Err(format!(
                "AUTH_MODE must be one of: disabled, oidc — got: {other:?}"
            )),
        }
    }

    pub fn is_disabled(self) -> bool {
        matches!(self, AuthMode::Disabled)
    }
}

impl fmt::Display for AuthMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthMode::Disabled => f.write_str("disabled"),
            AuthMode::Oidc => f.write_str("oidc"),
        }
    }
}

/// The fixed dev user injected when `AUTH_MODE=disabled`. Stable UUID so
/// integration tests can assert against it.
pub mod dev_user {
    use uuid::Uuid;

    pub const SESSION_ID: Uuid = Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
    pub const SUB: &str = "dev-local";
    pub const EMAIL: &str = "dev@local";
    pub const NAME: &str = "Dev User";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_disabled_variants() {
        assert_eq!(AuthMode::from_str("disabled").unwrap(), AuthMode::Disabled);
        assert_eq!(AuthMode::from_str("DISABLED").unwrap(), AuthMode::Disabled);
        assert_eq!(AuthMode::from_str("off").unwrap(), AuthMode::Disabled);
        assert_eq!(AuthMode::from_str("none").unwrap(), AuthMode::Disabled);
        assert_eq!(
            AuthMode::from_str("  disabled  ").unwrap(),
            AuthMode::Disabled
        );
    }

    #[test]
    fn parses_oidc() {
        assert_eq!(AuthMode::from_str("oidc").unwrap(), AuthMode::Oidc);
        assert_eq!(AuthMode::from_str("OIDC").unwrap(), AuthMode::Oidc);
    }

    #[test]
    fn rejects_unknown() {
        assert!(AuthMode::from_str("bogus").is_err());
        assert!(AuthMode::from_str("").is_err());
    }
}

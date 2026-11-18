//! Top-level auth mode toggle. Controls whether the server enforces OIDC
//! authentication or runs with auth disabled (dev/test instances).

use std::fmt;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AuthMode {
    Disabled,
    Oidc,
}

impl AuthMode {
    /// Parse `AUTH_MODE` from the environment. Defaults to `Oidc` if unset, so a
    /// deployment that forgets to configure auth fails closed instead of open.
    /// Returns Err on an unrecognized value.
    pub fn from_env() -> Result<Self, String> {
        Self::parse_or_default(std::env::var("AUTH_MODE").ok().as_deref())
    }

    fn parse_or_default(raw: Option<&str>) -> Result<Self, String> {
        raw.unwrap_or("oidc").parse()
    }

    /// The mode this server was started in, from managed state. Fails closed:
    /// if state is somehow missing, auth is enforced.
    pub fn of(req: &rocket::Request<'_>) -> Self {
        req.rocket()
            .state::<AuthMode>()
            .copied()
            .unwrap_or(AuthMode::Oidc)
    }

    pub fn is_disabled(self) -> bool {
        matches!(self, AuthMode::Disabled)
    }
}

impl std::str::FromStr for AuthMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "disabled" | "off" | "none" => Ok(AuthMode::Disabled),
            "oidc" => Ok(AuthMode::Oidc),
            other => Err(format!(
                "AUTH_MODE must be one of: disabled, oidc — got: {other:?}"
            )),
        }
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
        assert_eq!("disabled".parse::<AuthMode>().unwrap(), AuthMode::Disabled);
        assert_eq!("DISABLED".parse::<AuthMode>().unwrap(), AuthMode::Disabled);
        assert_eq!("off".parse::<AuthMode>().unwrap(), AuthMode::Disabled);
        assert_eq!("none".parse::<AuthMode>().unwrap(), AuthMode::Disabled);
        assert_eq!(
            "  disabled  ".parse::<AuthMode>().unwrap(),
            AuthMode::Disabled
        );
    }

    #[test]
    fn parses_oidc() {
        assert_eq!("oidc".parse::<AuthMode>().unwrap(), AuthMode::Oidc);
        assert_eq!("OIDC".parse::<AuthMode>().unwrap(), AuthMode::Oidc);
    }

    #[test]
    fn unset_defaults_to_enforced_auth() {
        assert_eq!(AuthMode::parse_or_default(None).unwrap(), AuthMode::Oidc);
        assert_eq!(
            AuthMode::parse_or_default(Some("disabled")).unwrap(),
            AuthMode::Disabled
        );
    }

    #[test]
    fn rejects_unknown() {
        assert!("bogus".parse::<AuthMode>().is_err());
        assert!("".parse::<AuthMode>().is_err());
    }
}

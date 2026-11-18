//! Refuse to start with a configuration that is unsafe on a public host.
//! Pure over its inputs so every rule is unit-tested.

use crate::auth_mode::AuthMode;

fn truthy(v: Option<String>) -> bool {
    matches!(
        v.as_deref()
            .map(|s| s.trim().to_ascii_lowercase())
            .as_deref(),
        Some("1" | "true" | "yes")
    )
}

fn is_local_host(url: &str) -> bool {
    url::Url::parse(url)
        .ok()
        .and_then(|u| {
            u.host_str()
                .map(|h| matches!(h, "localhost" | "127.0.0.1" | "[::1]"))
        })
        .unwrap_or(false)
}

/// Whether the session cookie should carry the `Secure` flag. True for any https
/// deployment; plain http is only tolerated for localhost development.
pub fn cookie_secure(redirect_url: &str) -> bool {
    !redirect_url.starts_with("http://")
}

/// Every reason this configuration is unsafe to run. Empty means fine.
/// `secret_key_configured` is whether Rocket has a real (non-zero) secret key.
pub fn problems(
    mode: AuthMode,
    get: impl Fn(&str) -> Option<String>,
    secret_key_configured: bool,
) -> Vec<String> {
    let mut out = Vec::new();
    match mode {
        AuthMode::Disabled => {
            if !truthy(get("AUTH_DISABLED_ACK")) {
                out.push(
                    "AUTH_MODE=disabled gives everyone full access. If this is a local/dev/test \
                     instance, set AUTH_DISABLED_ACK=true to confirm; otherwise set AUTH_MODE=oidc."
                        .to_string(),
                );
            }
        }
        AuthMode::Oidc => {
            let frontend = get("FRONTEND_URL").unwrap_or_default();
            if frontend.trim().is_empty() {
                out.push(
                    "FRONTEND_URL must be set (comma-separated allowed origins); an empty value \
                     would allow credentialed requests from any website."
                        .to_string(),
                );
            } else if frontend.split(',').any(|o| {
                let o = o.trim();
                o == "*" || !(o.starts_with("http://") || o.starts_with("https://"))
            }) {
                out.push(format!(
                    "FRONTEND_URL must be http(s) origins, not wildcards or bare hosts: {frontend:?}"
                ));
            }

            let key = get("ROCKET_SECRET_KEY").unwrap_or_default();
            if !secret_key_configured || key.trim().is_empty() || key.starts_with("CHANGE_ME") {
                out.push(
                    "ROCKET_SECRET_KEY must be set to a real secret (openssl rand -base64 32); \
                     it signs session cookies."
                        .to_string(),
                );
            }

            let redirect = get("AUTHENTIK_REDIRECT_URL").unwrap_or_default();
            if redirect.starts_with("http://") && !is_local_host(&redirect) {
                out.push(format!(
                    "AUTHENTIK_REDIRECT_URL must be https outside localhost (got {redirect:?})"
                ));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let m: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |k| m.get(k).cloned()
    }

    const GOOD: &[(&str, &str)] = &[
        ("FRONTEND_URL", "https://tilt.example.com"),
        (
            "ROCKET_SECRET_KEY",
            "c2VjcmV0LXNlY3JldC1zZWNyZXQtc2VjcmV0LXNlY3JldA==",
        ),
        (
            "AUTHENTIK_REDIRECT_URL",
            "https://tilt.example.com/api/v1/auth/callback",
        ),
    ];

    #[test]
    fn a_proper_oidc_config_is_fine() {
        assert!(problems(AuthMode::Oidc, env(GOOD), true).is_empty());
    }

    #[test]
    fn disabled_auth_needs_explicit_acknowledgement() {
        assert_eq!(problems(AuthMode::Disabled, env(&[]), true).len(), 1);
        assert_eq!(
            problems(
                AuthMode::Disabled,
                env(&[("AUTH_DISABLED_ACK", "false")]),
                true
            )
            .len(),
            1
        );
        assert!(
            problems(
                AuthMode::Disabled,
                env(&[("AUTH_DISABLED_ACK", "true")]),
                true
            )
            .is_empty()
        );
    }

    #[test]
    fn empty_or_wildcard_frontend_is_rejected_in_oidc_mode() {
        for bad in ["", "  ", "*", "tilt.example.com", "https://a.com,*"] {
            let mut e: Vec<(&str, &str)> = GOOD
                .iter()
                .filter(|(k, _)| *k != "FRONTEND_URL")
                .copied()
                .collect();
            e.push(("FRONTEND_URL", bad));
            assert_eq!(problems(AuthMode::Oidc, env(&e), true).len(), 1, "{bad:?}");
        }
        assert!(
            problems(
                AuthMode::Oidc,
                env(&[("FRONTEND_URL", "https://a.com, https://b.com")]
                    .iter()
                    .chain(&GOOD[1..])
                    .copied()
                    .collect::<Vec<_>>()),
                true
            )
            .is_empty()
        );
    }

    #[test]
    fn placeholder_or_missing_secret_key_is_rejected() {
        for key in ["", "CHANGE_ME_GENERATE_WITH_openssl_rand_base64_32"] {
            let mut e: Vec<(&str, &str)> = GOOD
                .iter()
                .filter(|(k, _)| *k != "ROCKET_SECRET_KEY")
                .copied()
                .collect();
            e.push(("ROCKET_SECRET_KEY", key));
            assert_eq!(problems(AuthMode::Oidc, env(&e), true).len(), 1, "{key:?}");
        }
        assert_eq!(
            problems(AuthMode::Oidc, env(GOOD), false).len(),
            1,
            "rocket has no key"
        );
    }

    #[test]
    fn plain_http_redirect_only_allowed_on_localhost() {
        let with = |r: &'static str| -> Vec<(&'static str, &'static str)> {
            let mut e: Vec<_> = GOOD
                .iter()
                .filter(|(k, _)| *k != "AUTHENTIK_REDIRECT_URL")
                .copied()
                .collect();
            e.push(("AUTHENTIK_REDIRECT_URL", r));
            e
        };
        assert_eq!(
            problems(
                AuthMode::Oidc,
                env(&with("http://tilt.example.com/cb")),
                true
            )
            .len(),
            1
        );
        assert!(problems(AuthMode::Oidc, env(&with("http://localhost:8000/cb")), true).is_empty());
    }

    #[test]
    fn cookie_is_secure_except_for_plain_http() {
        assert!(cookie_secure("https://tilt.example.com/cb"));
        assert!(!cookie_secure("http://localhost:8000/cb"));
    }
}

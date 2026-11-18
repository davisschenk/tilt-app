//! SSRF protection for user-supplied webhook URLs.
//!
//! Anyone with an account can point an alert target at any URL, and the server
//! makes the request. Without these checks that is a way to reach the host's own
//! loopback, the Docker network (Postgres), or cloud metadata (169.254.169.254).
//!
//! Defence in depth, because each layer alone has a gap:
//! - [`validate_url`] runs when a target is saved and again at dispatch time
//!   (covering rows saved before this existed). It is the only layer that sees
//!   IP-literal hosts, which never go through DNS.
//! - [`SafeResolver`] runs on the shared HTTP client and rejects any hostname
//!   that resolves to a non-public address, which also defeats DNS rebinding
//!   because the check happens on the addresses actually connected to.
//! - The client does not follow redirects, so a public URL cannot bounce the
//!   request to an internal one.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use reqwest::dns::{Addrs, Name, Resolve, Resolving};

/// Dev escape hatch: lets webhooks reach private addresses and use plain http,
/// e.g. a local receiver. Must stay off on a public instance.
pub fn allow_private() -> bool {
    std::env::var("WEBHOOK_ALLOW_PRIVATE")
        .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !(ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_multicast()
        || ip.is_documentation()
        || a == 0 // 0.0.0.0/8 "this network"
        || (a == 100 && (64..=127).contains(&b)) // 100.64.0.0/10 carrier-grade NAT
        || (a == 192 && b == 0 && c == 0) // 192.0.0.0/24 IETF protocol assignments
        || (a == 198 && (18..=19).contains(&b)) // 198.18.0.0/15 benchmarking
        || a >= 240) // 240.0.0.0/4 reserved
}

fn is_public_v6(ip: Ipv6Addr) -> bool {
    // IPv4 smuggled inside IPv6 (::ffff:127.0.0.1, 64:ff9b::7f00:1) is judged as that IPv4.
    if let Some(v4) = ip.to_ipv4_mapped() {
        return is_public_v4(v4);
    }
    let seg = ip.segments();
    if seg[0] == 0x0064 && seg[1] == 0xff9b && seg[2..6] == [0, 0, 0, 0] {
        let [hi, lo] = [seg[6].to_be_bytes(), seg[7].to_be_bytes()];
        return is_public_v4(Ipv4Addr::new(hi[0], hi[1], lo[0], lo[1]));
    }
    !(ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_multicast()
        || (seg[0] & 0xfe00) == 0xfc00 // fc00::/7 unique local
        || (seg[0] & 0xffc0) == 0xfe80 // fe80::/10 link local
        || (seg[0] == 0x2001 && seg[1] == 0x0db8)) // 2001:db8::/32 documentation
}

pub fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_public_v4(v4),
        IpAddr::V6(v6) => is_public_v6(v6),
    }
}

/// Check a webhook URL. Returns a short reason suitable for showing the user.
pub fn validate_url(raw: &str) -> Result<(), String> {
    let url = url::Url::parse(raw).map_err(|_| "not a valid URL".to_string())?;

    if allow_private() {
        return match url.scheme() {
            "http" | "https" => Ok(()),
            _ => Err("URL must start with http:// or https://".to_string()),
        };
    }

    if url.scheme() != "https" {
        return Err("webhook URLs must use https://".to_string());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("webhook URLs must not contain credentials".to_string());
    }
    match url.host() {
        None => Err("URL has no host".to_string()),
        Some(url::Host::Ipv4(ip)) if !is_public_v4(ip) => {
            Err("webhook URLs must not point at a private or internal address".to_string())
        }
        Some(url::Host::Ipv6(ip)) if !is_public_v6(ip) => {
            Err("webhook URLs must not point at a private or internal address".to_string())
        }
        Some(url::Host::Domain(d)) => {
            let d = d.trim_end_matches('.').to_ascii_lowercase();
            if d == "localhost"
                || d.ends_with(".localhost")
                || d.ends_with(".local")
                || d.ends_with(".internal")
                || !d.contains('.')
            {
                Err("webhook URLs must not point at a private or internal host".to_string())
            } else {
                Ok(())
            }
        }
        Some(_) => Ok(()),
    }
}

/// Resolves names like the system resolver, but only hands back public addresses.
pub struct SafeResolver;

impl Resolve for SafeResolver {
    fn resolve(&self, name: Name) -> Resolving {
        Box::pin(async move {
            let host = name.as_str().to_string();
            let found: Vec<SocketAddr> =
                tokio::net::lookup_host((host.as_str(), 0)).await?.collect();
            let allowed: Vec<SocketAddr> = if allow_private() {
                found
            } else {
                found.into_iter().filter(|a| is_public_ip(a.ip())).collect()
            };
            if allowed.is_empty() {
                return Err(format!("{host} does not resolve to a public address").into());
            }
            Ok(Box::new(allowed.into_iter()) as Addrs)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bad(url: &str) {
        assert!(validate_url(url).is_err(), "{url} should be rejected");
    }
    fn good(url: &str) {
        assert!(validate_url(url).is_ok(), "{url} should be allowed");
    }

    #[test]
    fn allows_ordinary_public_https_webhooks() {
        good("https://discord.com/api/webhooks/123/abc");
        good("https://hooks.slack.com/services/T0/B0/xxx");
        good("https://example.com:8443/hook?x=1");
        good("https://8.8.8.8/hook");
        good("https://[2606:4700:4700::1111]/hook");
    }

    #[test]
    fn rejects_plain_http_and_other_schemes() {
        bad("http://example.com/hook");
        bad("ftp://example.com/hook");
        bad("file:///etc/passwd");
        bad("gopher://example.com");
        bad("not a url");
    }

    #[test]
    fn rejects_loopback_private_and_metadata_addresses() {
        for ip in [
            "127.0.0.1",
            "127.1.2.3",
            "0.0.0.0",
            "10.0.0.5",
            "172.16.0.1",
            "172.31.255.255",
            "192.168.1.1",
            "169.254.169.254", // cloud metadata
            "100.64.0.1",      // carrier-grade NAT
            "198.18.0.1",
            "224.0.0.1",
            "255.255.255.255",
        ] {
            bad(&format!("https://{ip}/hook"));
        }
        bad("https://[::1]/hook");
        bad("https://[::]/hook");
        bad("https://[fe80::1]/hook");
        bad("https://[fd00::1]/hook");
        bad("https://[::ffff:127.0.0.1]/hook");
        bad("https://[::ffff:169.254.169.254]/hook");
        bad("https://[64:ff9b::a9fe:a9fe]/hook"); // NAT64 form of 169.254.169.254
    }

    #[test]
    fn rejects_internal_hostnames_and_credentials() {
        bad("https://localhost/hook");
        bad("https://LOCALHOST./hook");
        bad("https://app.localhost/hook");
        bad("https://db/hook"); // single-label docker service name
        bad("https://printer.local/hook");
        bad("https://vault.internal/hook");
        bad("https://user:pass@example.com/hook");
        bad("https://example.com@127.0.0.1/hook");
    }

    #[test]
    fn rejects_integer_and_octal_ip_tricks() {
        // The url crate normalises these to dotted quads before we look at them.
        bad("https://2130706433/hook"); // 127.0.0.1 as an integer
        bad("https://0x7f000001/hook");
        bad("https://0177.0.0.1/hook");
        bad("https://127.1/hook");
    }

    #[test]
    fn classifies_public_ips() {
        assert!(is_public_ip("8.8.8.8".parse().unwrap()));
        assert!(is_public_ip("1.1.1.1".parse().unwrap()));
        assert!(!is_public_ip("10.1.2.3".parse().unwrap()));
        assert!(!is_public_ip("::1".parse().unwrap()));
    }

    #[tokio::test]
    async fn resolver_refuses_hostnames_that_resolve_to_loopback() {
        // `localhost` passes URL validation only if allowed; go straight to the
        // client to prove the resolver layer stops it on its own.
        let client = reqwest::Client::builder()
            .dns_resolver(std::sync::Arc::new(SafeResolver))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        let err = client.get("https://localhost:9/").send().await.unwrap_err();
        let mut chain = format!("{err:?}");
        let mut src = std::error::Error::source(&err);
        while let Some(e) = src {
            chain.push_str(&format!(" {e}"));
            src = e.source();
        }
        assert!(
            chain.contains("public address"),
            "unexpected error: {chain}"
        );
    }
}

//! Pure OTA decision logic (no ESP-IDF dependencies, unit-testable on the host).
//!
//! The server advertises `{version, url, sha256, size, signature}`. The signature is an
//! Ed25519 signature over [`signed_message`], so a device with the matching public key
//! only installs images that the release pipeline produced, regardless of transport.

use ed25519_compact::{PublicKey, Signature};

/// Domain-separation prefix for the signed message.
const MESSAGE_PREFIX: &str = "tilt-ota-v1";

/// Parse `MAJOR.MINOR.PATCH` (an optional `-pre`/`+build` suffix is ignored).
pub fn parse_semver(v: &str) -> Option<(u32, u32, u32)> {
    let core = v.trim().split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

/// True only if `server` is strictly newer than `current`. Unparsable versions are
/// never newer, which prevents downgrades and mismatch-driven update loops.
pub fn is_newer(server: &str, current: &str) -> bool {
    match (parse_semver(server), parse_semver(current)) {
        (Some(s), Some(c)) => s > c,
        _ => false,
    }
}

/// The exact byte string covered by the signature. Binding version and size (not just
/// the hash) stops an attacker replaying an old signed image under a newer version.
pub fn signed_message(version: &str, size: u64, sha256_hex: &str) -> String {
    format!("{MESSAGE_PREFIX}:{version}:{size}:{}", sha256_hex.to_ascii_lowercase())
}

pub fn hex_decode(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.len() % 2 != 0 || !s.is_ascii() {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Verify the manifest signature against the compiled-in public key (hex).
pub fn verify_manifest(
    public_key_hex: &str,
    version: &str,
    size: u64,
    sha256_hex: &str,
    signature_hex: &str,
) -> Result<(), String> {
    let pk_bytes = hex_decode(public_key_hex).ok_or("public key is not valid hex")?;
    let pk = PublicKey::from_slice(&pk_bytes).map_err(|_| "public key has wrong length")?;
    let sig_bytes = hex_decode(signature_hex).ok_or("signature is not valid hex")?;
    let sig = Signature::from_slice(&sig_bytes).map_err(|_| "signature has wrong length")?;
    pk.verify(signed_message(version, size, sha256_hex).as_bytes(), &sig)
        .map_err(|_| "signature verification failed".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_compact::{KeyPair, Seed};

    const SHA: &str = "26e01a743bcfad9eba9318e4189e80daf3c548bdcfbaadc62be7d5ded4d62ab6";

    fn keypair(seed: u8) -> KeyPair {
        KeyPair::from_seed(Seed::new([seed; 32]))
    }

    fn sign(kp: &KeyPair, version: &str, size: u64, sha: &str) -> String {
        hex_encode(&*kp.sk.sign(signed_message(version, size, sha).as_bytes(), None))
    }

    #[test]
    fn semver_parsing() {
        assert_eq!(parse_semver("0.1.0"), Some((0, 1, 0)));
        assert_eq!(parse_semver("1.2.3-rc1"), Some((1, 2, 3)));
        assert_eq!(parse_semver("1.2"), None);
        assert_eq!(parse_semver("1.2.3.4"), None);
        assert_eq!(parse_semver("abc"), None);
    }

    #[test]
    fn only_strictly_newer_installs() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("0.10.0", "0.9.0"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.0.9", "0.1.0"));
        assert!(!is_newer("0.0.0", "0.1.0"));
        assert!(!is_newer("garbage", "0.1.0"));
    }

    #[test]
    fn valid_signature_accepted() {
        let kp = keypair(1);
        let sig = sign(&kp, "0.2.0", 1000, SHA);
        let pk = hex_encode(&*kp.pk);
        assert!(verify_manifest(&pk, "0.2.0", 1000, SHA, &sig).is_ok());
    }

    #[test]
    fn tampered_fields_rejected() {
        let kp = keypair(1);
        let sig = sign(&kp, "0.2.0", 1000, SHA);
        let pk = hex_encode(&*kp.pk);
        assert!(verify_manifest(&pk, "0.3.0", 1000, SHA, &sig).is_err());
        assert!(verify_manifest(&pk, "0.2.0", 1001, SHA, &sig).is_err());
        let other = SHA.replace('2', "3");
        assert!(verify_manifest(&pk, "0.2.0", 1000, &other, &sig).is_err());
    }

    #[test]
    fn wrong_key_and_malformed_input_rejected() {
        let kp = keypair(1);
        let sig = sign(&kp, "0.2.0", 1000, SHA);
        let wrong_pk = hex_encode(&*keypair(2).pk);
        assert!(verify_manifest(&wrong_pk, "0.2.0", 1000, SHA, &sig).is_err());
        let pk = hex_encode(&*kp.pk);
        assert!(verify_manifest(&pk, "0.2.0", 1000, SHA, "zz").is_err());
        assert!(verify_manifest(&pk, "0.2.0", 1000, SHA, "").is_err());
        assert!(verify_manifest("abcd", "0.2.0", 1000, SHA, &sig).is_err());
    }

    #[test]
    fn hex_round_trip() {
        assert_eq!(hex_decode(&hex_encode(&[0, 15, 255])), Some(vec![0, 15, 255]));
        assert_eq!(hex_decode("abc"), None);
    }
}

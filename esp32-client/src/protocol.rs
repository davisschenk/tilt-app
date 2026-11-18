//! Serial provisioning wire protocol (pure logic, unit-testable on the host).
//!
//! Requests are single lines `TILT>{json}`; replies are single lines `TILT<{json}`.
//! Everything else on the serial port is ordinary log output.

use serde::{Deserialize, Serialize};

pub const REQUEST_PREFIX: &str = "TILT>";
pub const REPLY_PREFIX: &str = "TILT<";

/// Longest request line we accept; anything longer is discarded.
pub const MAX_LINE_LEN: usize = 1024;

/// Fields a client may change. Absent fields keep their stored value.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfigPatch {
    pub wifi_ssid: Option<String>,
    pub wifi_pass: Option<String>,
    pub server_url: Option<String>,
    pub api_key: Option<String>,
    pub device_name: Option<String>,
    pub scan_interval_secs: Option<u32>,
    pub min_samples: Option<u32>,
    pub ewa_alpha: Option<u32>,
    pub ota_check_interval_cycles: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Request {
    Info,
    GetConfig,
    SetConfig { config: ConfigPatch },
    WifiScan,
    FactoryReset,
}

/// Extract and parse the request embedded in a serial line, if any.
/// Returns `None` for ordinary log lines, `Some(Err(..))` for malformed requests.
pub fn parse_line(line: &str) -> Option<Result<Request, String>> {
    let idx = line.rfind(REQUEST_PREFIX)?;
    let json = line[idx + REQUEST_PREFIX.len()..].trim();
    Some(serde_json::from_str(json).map_err(|e| format!("bad request: {e}")))
}

/// Format a reply line (without trailing newline).
pub fn format_reply(cmd: &str, mut body: serde_json::Value) -> String {
    if let Some(obj) = body.as_object_mut() {
        obj.insert("cmd".into(), cmd.into());
    }
    format!("{REPLY_PREFIX}{body}")
}

/// Mask a secret for display: empty stays empty, anything else becomes `***`.
pub fn mask_secret(s: &str) -> &'static str {
    if s.is_empty() { "" } else { "***" }
}

/// Assembles newline-delimited lines from raw serial bytes.
#[derive(Default)]
pub struct LineAssembler {
    buf: Vec<u8>,
    overflow: bool,
}

impl LineAssembler {
    /// Feed bytes, returning every line completed by them (CR/LF stripped).
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<String> {
        let mut out = Vec::new();
        for &b in bytes {
            if b == b'\n' {
                if !self.overflow {
                    let line = String::from_utf8_lossy(&self.buf).trim_end_matches('\r').to_string();
                    if !line.is_empty() {
                        out.push(line);
                    }
                }
                self.buf.clear();
                self.overflow = false;
            } else if !self.overflow {
                if self.buf.len() >= MAX_LINE_LEN {
                    self.overflow = true;
                    self.buf.clear();
                } else {
                    self.buf.push(b);
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_commands() {
        assert_eq!(parse_line("TILT>{\"cmd\":\"info\"}"), Some(Ok(Request::Info)));
        assert_eq!(parse_line("TILT>{\"cmd\":\"wifi_scan\"}"), Some(Ok(Request::WifiScan)));
        assert_eq!(parse_line("TILT>{\"cmd\":\"get_config\"}"), Some(Ok(Request::GetConfig)));
        assert_eq!(parse_line("TILT>{\"cmd\":\"factory_reset\"}"), Some(Ok(Request::FactoryReset)));
    }

    #[test]
    fn parses_set_config_with_partial_fields() {
        let line = r#"TILT>{"cmd":"set_config","config":{"wifiSsid":"home","scanIntervalSecs":30}}"#;
        match parse_line(line) {
            Some(Ok(Request::SetConfig { config })) => {
                assert_eq!(config.wifi_ssid.as_deref(), Some("home"));
                assert_eq!(config.scan_interval_secs, Some(30));
                assert!(config.wifi_pass.is_none());
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn request_may_follow_log_noise() {
        let line = "I (1234) tilt: hello TILT>{\"cmd\":\"info\"}";
        assert_eq!(parse_line(line), Some(Ok(Request::Info)));
    }

    #[test]
    fn ignores_plain_logs_and_rejects_garbage() {
        assert!(parse_line("I (1) wifi: connected").is_none());
        assert!(matches!(parse_line("TILT>{nope"), Some(Err(_))));
        assert!(matches!(parse_line("TILT>{\"cmd\":\"explode\"}"), Some(Err(_))));
        assert!(matches!(
            parse_line(r#"TILT>{"cmd":"set_config","config":{"bogus":1}}"#),
            Some(Err(_))
        ));
    }

    #[test]
    fn reply_includes_cmd() {
        let s = format_reply("info", serde_json::json!({"ok": true}));
        assert!(s.starts_with("TILT<"));
        assert!(s.contains("\"cmd\":\"info\""));
    }

    #[test]
    fn masks_secrets() {
        assert_eq!(mask_secret(""), "");
        assert_eq!(mask_secret("hunter2"), "***");
    }

    #[test]
    fn assembler_handles_split_and_multiple_lines() {
        let mut a = LineAssembler::default();
        assert!(a.feed(b"TILT>{\"cmd\"").is_empty());
        let lines = a.feed(b":\"info\"}\r\nsecond\nthi");
        assert_eq!(lines, vec!["TILT>{\"cmd\":\"info\"}".to_string(), "second".to_string()]);
        assert_eq!(a.feed(b"rd\n"), vec!["third".to_string()]);
    }

    #[test]
    fn assembler_drops_oversized_lines() {
        let mut a = LineAssembler::default();
        let big = vec![b'x'; MAX_LINE_LEN + 10];
        assert!(a.feed(&big).is_empty());
        assert!(a.feed(b"\n").is_empty());
        assert_eq!(a.feed(b"ok\n"), vec!["ok".to_string()]);
    }
}

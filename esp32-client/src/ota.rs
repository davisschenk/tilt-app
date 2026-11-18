//! OTA firmware update module.
//!
//! Downloads a firmware binary from a URL and flashes it to the inactive OTA slot.
//! The manifest (version, size, SHA-256, Ed25519 signature) is verified *before* any
//! flash write, and the streamed image hash is checked before the slot is activated.
//! The caller is responsible for triggering a reboot after a successful update.

use anyhow::{Context, Result};
use esp_idf_svc::http::client::{Configuration as HttpConfig, EspHttpConnection};
use esp_idf_svc::ota::EspOta;
use sha2::{Digest, Sha256};

use crate::ota_logic;

/// Firmware download chunk size in bytes. Heap-allocated: an 8 KiB array on the stack
/// would consume the entire 8 KiB main task stack.
const CHUNK_SIZE: usize = 8192;

/// Log a progress message every N chunks (~80 KiB per message at 8 KiB chunks).
const LOG_EVERY_N_CHUNKS: usize = 10;

/// Per-read network timeout.
const HTTP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// What the server claims about the image; authenticated by `signature`.
pub struct OtaTarget<'a> {
    pub version: &'a str,
    pub size: u64,
    pub sha256_hex: &'a str,
    pub signature_hex: &'a str,
}

/// Performs OTA firmware updates by downloading and flashing to the inactive slot.
pub struct OtaUpdater;

impl OtaUpdater {
    /// Verify `target` against `public_key_hex`, then download `url` into the inactive slot.
    ///
    /// On success the new slot is set as the next boot target. On any error the update is
    /// aborted and the current firmware is left intact. Does NOT reboot.
    pub fn perform_update(url: &str, target: &OtaTarget<'_>, public_key_hex: &str) -> Result<()> {
        ota_logic::verify_manifest(
            public_key_hex,
            target.version,
            target.size,
            target.sha256_hex,
            target.signature_hex,
        )
        .map_err(|e| anyhow::anyhow!("OTA: manifest rejected: {e}"))?;

        let slot_size = Self::next_slot_size()?;
        if target.size == 0 || target.size > slot_size {
            anyhow::bail!(
                "OTA: image size {} does not fit update slot ({} bytes)",
                target.size,
                slot_size
            );
        }

        log::info!("OTA: manifest verified, downloading {} from {}", target.version, url);

        let mut ota = EspOta::new().context("OTA: failed to initialise EspOta")?;
        let mut ota_update = ota
            .initiate_update()
            .context("OTA: failed to initiate update")?;

        match Self::download_and_write(url, target, &mut ota_update) {
            Ok(()) => {
                let finished = ota_update
                    .finish()
                    .context("OTA: failed to finish update")?;
                finished
                    .activate()
                    .context("OTA: failed to activate new slot")?;
                log::info!("OTA: new firmware activated, ready to reboot");
                Ok(())
            }
            Err(e) => {
                log::error!("OTA: download/verify failed: {:?}, aborting", e);
                if let Err(abort_err) = ota_update.abort() {
                    log::warn!("OTA: abort also failed: {:?}", abort_err);
                }
                Err(e)
            }
        }
    }

    /// Size in bytes of the partition the next update will be written to.
    fn next_slot_size() -> Result<u64> {
        // Safety: esp_ota_get_next_update_partition(NULL) returns a pointer into the
        // static partition table (or NULL); we only read `size` after a null check.
        let part = unsafe { esp_idf_svc::sys::esp_ota_get_next_update_partition(core::ptr::null()) };
        if part.is_null() {
            anyhow::bail!("OTA: no update partition available");
        }
        Ok(unsafe { (*part).size } as u64)
    }

    /// Stream firmware into `ota_update`, hashing as we go. Fails if the stream exceeds
    /// the declared size, ends short, or its SHA-256 differs from the signed value.
    fn download_and_write(
        url: &str,
        target: &OtaTarget<'_>,
        ota_update: &mut esp_idf_svc::ota::EspOtaUpdate<'_>,
    ) -> Result<()> {
        let config = HttpConfig {
            buffer_size: Some(CHUNK_SIZE),
            buffer_size_tx: Some(1024),
            timeout: Some(HTTP_TIMEOUT),
            // Attaches the ESP-IDF CA bundle for HTTPS URLs; harmless for plain HTTP.
            crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
            ..Default::default()
        };

        let mut client =
            EspHttpConnection::new(&config).context("OTA: failed to create HTTP client")?;

        client
            .initiate_request(esp_idf_svc::http::Method::Get, url, &[("Accept", "*/*")])
            .context("OTA: failed to initiate HTTP request")?;
        client
            .initiate_response()
            .context("OTA: failed to initiate HTTP response")?;

        let status = client.status();
        if status != 200 {
            anyhow::bail!("OTA: server returned HTTP {}", status);
        }

        let mut buf = vec![0u8; CHUNK_SIZE];
        let mut hasher = Sha256::new();
        let mut total_bytes: u64 = 0;
        let mut chunk_count: usize = 0;

        loop {
            let n = client
                .read(&mut buf)
                .context("OTA: error reading firmware data")?;
            if n == 0 {
                break;
            }

            total_bytes += n as u64;
            if total_bytes > target.size {
                anyhow::bail!("OTA: image larger than declared size {}", target.size);
            }

            hasher.update(&buf[..n]);
            ota_update
                .write(&buf[..n])
                .context("OTA: error writing chunk to flash")?;

            chunk_count += 1;
            if chunk_count % LOG_EVERY_N_CHUNKS == 0 {
                log::info!("OTA: written {} KB so far...", total_bytes / 1024);
                // A ~1.5 MB image can take tens of seconds on a slow link.
                crate::feed_watchdog_or_warn();
            }
        }

        if total_bytes != target.size {
            anyhow::bail!("OTA: got {} bytes, expected {}", total_bytes, target.size);
        }
        let digest = ota_logic::hex_encode(&hasher.finalize());
        if !digest.eq_ignore_ascii_case(target.sha256_hex) {
            anyhow::bail!("OTA: image hash {} does not match signed hash", digest);
        }
        log::info!("OTA: download complete ({} bytes), hash verified", total_bytes);
        Ok(())
    }
}

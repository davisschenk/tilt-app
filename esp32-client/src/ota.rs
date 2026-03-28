//! OTA firmware update module.
//!
//! Downloads a firmware binary from a URL and flashes it to the inactive OTA slot.
//! The caller is responsible for triggering a reboot after a successful update.

use anyhow::{Context, Result};
use esp_idf_svc::http::client::{Configuration as HttpConfig, EspHttpConnection};
use esp_idf_svc::ota::EspOta;

/// Firmware download chunk size in bytes. Matches the HTTP receive buffer to
/// avoid double-buffering and keep heap usage predictable.
const CHUNK_SIZE: usize = 8192;

/// Log a progress message every N chunks (~80 KiB per message at 8 KiB chunks).
const LOG_EVERY_N_CHUNKS: usize = 10;

/// Performs OTA firmware updates by downloading and flashing to the inactive slot.
pub struct OtaUpdater;

impl OtaUpdater {
    /// Download firmware from `url` and flash it to the inactive OTA slot.
    ///
    /// On success the new slot is marked valid and set as the next boot target.
    /// On any error the update is aborted and the current firmware is left intact.
    /// Does NOT reboot — the caller must trigger a reboot when ready.
    ///
    /// `url` must be a validated HTTP/HTTPS URL (validated at call site in main).
    pub fn perform_update(url: &str) -> Result<()> {
        log::info!("OTA: starting firmware update from {}", url);

        let mut ota = EspOta::new().context("OTA: failed to initialise EspOta")?;

        let update_slot = ota
            .get_update_slot()
            .context("OTA: failed to get update slot info")?;
        log::info!(
            "OTA: writing to slot label='{}' state={:?}",
            update_slot.label,
            update_slot.state
        );

        let mut ota_update = ota
            .initiate_update()
            .context("OTA: failed to initiate update")?;

        let result = Self::download_and_write(url, &mut ota_update);

        match result {
            Ok(bytes_written) => {
                log::info!("OTA: download complete ({} bytes), activating slot", bytes_written);
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
                log::error!("OTA: download/write failed: {:?}, aborting", e);
                if let Err(abort_err) = ota_update.abort() {
                    log::warn!("OTA: abort also failed: {:?}", abort_err);
                }
                Err(e)
            }
        }
    }

    /// Stream firmware from `url` directly into `ota_update` in `CHUNK_SIZE` chunks.
    ///
    /// Returns the total number of bytes written on success.
    fn download_and_write(
        url: &str,
        ota_update: &mut esp_idf_svc::ota::EspOtaUpdate<'_>,
    ) -> Result<usize> {
        let config = HttpConfig {
            buffer_size: Some(CHUNK_SIZE),
            buffer_size_tx: Some(1024),
            // Required for HTTPS firmware URLs — attaches the ESP-IDF bundled CA
            // certificate store to the TLS context. Harmless for plain HTTP.
            crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
            ..Default::default()
        };

        let mut client =
            EspHttpConnection::new(&config).context("OTA: failed to create HTTP client")?;

        client
            .initiate_request(
                esp_idf_svc::http::Method::Get,
                url,
                &[("Accept", "*/*")],
            )
            .context("OTA: failed to initiate HTTP request")?;

        client
            .initiate_response()
            .context("OTA: failed to initiate HTTP response")?;

        let status = client.status();
        if status != 200 {
            anyhow::bail!("OTA: server returned HTTP {}", status);
        }

        let mut buf = [0u8; CHUNK_SIZE];
        let mut total_bytes: usize = 0;
        let mut chunk_count: usize = 0;

        loop {
            let n = client
                .read(&mut buf)
                .context("OTA: error reading firmware data")?;

            if n == 0 {
                break;
            }

            ota_update
                .write(&buf[..n])
                .context("OTA: error writing chunk to flash")?;

            total_bytes += n;
            chunk_count += 1;

            if chunk_count % LOG_EVERY_N_CHUNKS == 0 {
                log::info!("OTA: written {} KB so far...", total_bytes / 1024);
                // Feed watchdog during the download — a 1 MB firmware is ~125
                // chunks at 8 KB each and can take tens of seconds on a slow link.
                crate::feed_watchdog_or_warn();
            }
        }

        Ok(total_bytes)
    }
}

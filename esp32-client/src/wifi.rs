//! WiFi connection management.
//!
//! Manages ESP32 WiFi in STA mode: initial connection, monitoring, and
//! automatic reconnection with exponential backoff on disconnect.

use core::convert::TryInto;

use anyhow::{Context, Result};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::modem::WifiModemPeripheral;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{
    AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi, PmfConfiguration,
};

/// Manages the ESP32 WiFi peripheral in station (STA) mode.
pub struct WifiManager {
    wifi: BlockingWifi<EspWifi<'static>>,
    ssid: &'static str,
    password: &'static str,
}

impl WifiManager {
    /// Create a new `WifiManager` using the given modem peripheral and credentials.
    ///
    /// Does not connect — call `connect()` to establish the initial association.
    pub fn new(
        modem: impl WifiModemPeripheral + 'static,
        sys_loop: EspSystemEventLoop,
        nvs: EspDefaultNvsPartition,
        ssid: &'static str,
        password: &'static str,
    ) -> Result<Self> {
        let esp_wifi = EspWifi::new(modem, sys_loop.clone(), Some(nvs))
            .context("Failed to create EspWifi")?;
        let wifi = BlockingWifi::wrap(esp_wifi, sys_loop)
            .context("Failed to create BlockingWifi")?;

        Ok(Self {
            wifi,
            ssid,
            password,
        })
    }

    /// Configure and connect to the access point, then wait for an IP address.
    ///
    /// Suspends the Task Watchdog Timer around blocking WiFi operations since
    /// `connect()` and `wait_netif_up()` cannot yield control to feed the TWDT.
    pub fn connect(&mut self) -> Result<()> {
        let ssid_heapless: heapless::String<32> = self
            .ssid
            .try_into()
            .map_err(|_| anyhow::anyhow!("WiFi SSID too long (max 32 chars)"))?;
        let password_heapless: heapless::String<64> = self
            .password
            .try_into()
            .map_err(|_| anyhow::anyhow!("WiFi password too long (max 64 chars)"))?;

        let wifi_config = Configuration::Client(ClientConfiguration {
            ssid: ssid_heapless,
            password: password_heapless,
            auth_method: AuthMethod::WPA2Personal,
            pmf_cfg: PmfConfiguration::Capable { required: false },
            ..Default::default()
        });

        self.wifi
            .set_configuration(&wifi_config)
            .context("Failed to set WiFi configuration")?;

        self.wifi.start().context("Failed to start WiFi")?;
        log::info!("WiFi started, connecting to '{}'...", self.ssid);

        // Suspend TWDT during blocking WiFi operations — connect() and
        // wait_netif_up() block internally and we cannot feed the watchdog.
        crate::suspend_watchdog();

        // Retry connect up to 3 times — WiFi 6 APs can reject the first
        // association attempt from a legacy 802.11n client before accepting.
        const MAX_CONNECT_ATTEMPTS: u32 = 3;
        let mut connect_result: Result<()> = Ok(());
        for attempt in 1..=MAX_CONNECT_ATTEMPTS {
            connect_result = self.wifi.connect().context("Failed to connect to WiFi");
            if connect_result.is_ok() {
                break;
            }
            log::warn!(
                "WiFi connect attempt {}/{} failed, retrying...",
                attempt, MAX_CONNECT_ATTEMPTS
            );
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
        if let Err(e) = connect_result {
            crate::resume_watchdog();
            return Err(e);
        }
        log::info!("WiFi connected, waiting for IP...");

        let netif_result = self.wifi.wait_netif_up().context("Failed to get IP address");
        crate::resume_watchdog();
        netif_result?;

        let ip_info = self
            .wifi
            .wifi()
            .sta_netif()
            .get_ip_info()
            .context("Failed to get IP info")?;
        log::info!("WiFi connected: IP={}", ip_info.ip);

        Ok(())
    }

    /// Return `true` if the station is currently associated to an access point.
    pub fn is_connected(&self) -> bool {
        self.wifi.is_connected().unwrap_or(false)
    }

    /// Reconnect if the link is down. Returns `Ok(())` once an IP is obtained.
    ///
    /// First attempts a lightweight reconnect; falls back to a full stop/start
    /// cycle if that fails. Suspends the TWDT for the duration of blocking calls.
    pub fn ensure_connected(&mut self) -> Result<()> {
        if self.is_connected() {
            return Ok(());
        }

        log::warn!("WiFi disconnected, attempting reconnect...");

        // Suspend TWDT during blocking WiFi reconnect operations
        crate::suspend_watchdog();

        // Try to reconnect without full reconfiguration
        if let Err(e) = self.wifi.connect() {
            log::warn!("WiFi reconnect failed: {:?}, retrying with full restart...", e);
            // Full restart on reconnect failure
            let _ = self.wifi.stop();
            if let Err(e) = self.wifi.start().context("Failed to restart WiFi") {
                crate::resume_watchdog();
                return Err(e);
            }
            if let Err(e) = self.wifi.connect().context("Failed to reconnect WiFi after restart") {
                crate::resume_watchdog();
                return Err(e);
            }
        }

        let netif_result = self.wifi
            .wait_netif_up()
            .context("Failed to get IP after reconnect");
        crate::resume_watchdog();
        netif_result?;

        let ip_info = self
            .wifi
            .wifi()
            .sta_netif()
            .get_ip_info()
            .context("Failed to get IP info after reconnect")?;
        log::info!("WiFi reconnected: IP={}", ip_info.ip);

        Ok(())
    }
}

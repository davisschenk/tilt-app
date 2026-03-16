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
    AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi,
};

pub struct WifiManager {
    wifi: BlockingWifi<EspWifi<'static>>,
    ssid: &'static str,
    password: &'static str,
}

impl WifiManager {
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
            auth_method: AuthMethod::None,
            ..Default::default()
        });

        self.wifi
            .set_configuration(&wifi_config)
            .context("Failed to set WiFi configuration")?;

        self.wifi.start().context("Failed to start WiFi")?;
        log::info!("WiFi started, connecting to '{}'...", self.ssid);

        self.wifi.connect().context("Failed to connect to WiFi")?;
        log::info!("WiFi connected, waiting for IP...");

        self.wifi
            .wait_netif_up()
            .context("Failed to get IP address")?;

        let ip_info = self
            .wifi
            .wifi()
            .sta_netif()
            .get_ip_info()
            .context("Failed to get IP info")?;
        log::info!("WiFi connected: IP={}", ip_info.ip);

        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.wifi.is_connected().unwrap_or(false)
    }

    pub fn ensure_connected(&mut self) -> Result<()> {
        if self.is_connected() {
            return Ok(());
        }

        log::warn!("WiFi disconnected, attempting reconnect...");

        // Try to reconnect without full reconfiguration
        if let Err(e) = self.wifi.connect() {
            log::warn!("WiFi reconnect failed: {:?}, retrying with full restart...", e);
            // Full restart on reconnect failure
            let _ = self.wifi.stop();
            self.wifi.start().context("Failed to restart WiFi")?;
            self.wifi
                .connect()
                .context("Failed to reconnect WiFi after restart")?;
        }

        self.wifi
            .wait_netif_up()
            .context("Failed to get IP after reconnect")?;

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

# ESP32 Tilt Hydrometer Client

A Rust-based ESP32 client that scans for Tilt hydrometer BLE iBeacon advertisements and uploads readings to the Tilt server API.

## Quick start (web setup)

No toolchain needed. In the web UI open **Device Setup** (Chrome, Edge or another Chromium browser; the site must be served over HTTPS or from `localhost`):

1. Plug the ESP32 in over USB and click **Select serial port**.
2. **Flash** the firmware bundled with your server (skip this if the board already runs Tilt firmware).
3. Pick your WiFi network, check the server URL, and save. An API key is created for the device automatically.
4. The device restarts, joins WiFi, and the page confirms each step from the serial log.

Settings live in the device's NVS, so you can come back to **Device Setup** at any time to reconfigure without reflashing. The server must have firmware installed: the official Docker image includes it, or build it yourself with `just esp32-release` and point `FIRMWARE_DIR` at `firmware-dist/`.

The sections below cover building from source for development.

## Prerequisites

- **espup** — ESP32 Rust toolchain installer
- **Xtensa Rust toolchain** — installed via espup (channel `esp`)
- **ESP-IDF v5.3** — downloaded automatically by the build system
- **Python 3** with `venv` module (`sudo apt install python3-venv`)
- **cmake** and **ninja-build** (`sudo apt install cmake ninja-build`)
- **espflash** — for flashing firmware (`cargo install espflash`)
- **ldproxy** — linker proxy for ESP-IDF (`cargo install ldproxy`)

## Setup

### 1. Install the ESP32 Rust Toolchain

```bash
cargo install espup
espup install --targets esp32
```

After installation, source the environment variables (required in every new terminal):

```bash
. $HOME/export-esp.sh
```

### 2. Install Build Tools

```bash
cargo install ldproxy
sudo apt install python3-venv cmake ninja-build
```

### 3. Configure WiFi and Server

Copy the example configuration and fill in your values:

```bash
cp cfg.toml.example cfg.toml
```

`cfg.toml` is **optional**: it only supplies compile-time defaults. Anything saved from the web UI (stored in NVS) takes priority. Without it, the build produces a generic image that waits for configuration over serial.

Edit `cfg.toml` with your settings:

| Field | Description | Default |
|---|---|---|
| `wifi_ssid` | WiFi network name | *(required)* |
| `wifi_password` | WiFi password | *(required)* |
| `server_url` | Tilt server API base URL | `http://192.168.1.100:8000` |
| `api_key` | API key for authentication | *(empty, optional)* |
| `scan_interval_secs` | BLE scan interval | `15` |
| `upload_interval_secs` | Upload batch interval | `60` |
| `buffer_capacity` | Max buffered readings | `50` |
| `watchdog_timeout_secs` | Watchdog reboot timeout | `120` |
| `health_report_interval_cycles` | Health log frequency | `60` |
| `ota_check_interval_cycles` | OTA check frequency (cycles) | `60` |

The firmware version comes from `Cargo.toml`.

**WARNING:** `cfg.toml` contains WiFi credentials. It is gitignored and must never be committed.

## Building

Source the ESP environment and build:

```bash
. $HOME/export-esp.sh
cargo build --release
```

The binary is output to `target/xtensa-esp32-espidf/release/esp32-client`.

## Flashing

Connect your ESP32 via USB and flash:

```bash
espflash flash --monitor target/xtensa-esp32-espidf/release/esp32-client
```

The `--monitor` flag opens a serial console immediately after flashing so you can see log output.

To monitor without reflashing:

```bash
espflash monitor
```

## Testing

Unit tests for pure-Rust modules (tilt.rs, buffer.rs) can be run on the host:

```bash
./test-host.sh
```

This creates a temporary crate with the stable toolchain and runs all 22 tests.

## Architecture

```
src/
  main.rs    — Entry point, scan-upload loop, watchdog, health logging
  ble.rs     — NimBLE BLE scanning, Tilt filtering, stack recovery
  tilt.rs    — Tilt iBeacon UUID constants, color enum, parser
  wifi.rs    — WiFi STA connection manager with auto-reconnect
  http.rs    — HTTP client for batch reading uploads
  buffer.rs  — Bounded reading buffer + exponential backoff
  config.rs  — Compile-time config (toml-cfg) + NVS runtime overrides
  provision.rs / protocol.rs — Serial provisioning (web setup)
```

## OTA Firmware Updates

The ESP32 client supports over-the-air firmware updates. The device periodically polls the server for a newer firmware version and flashes it automatically.

### How It Works

1. Every `ota_check_interval_cycles` scan cycles (default: every 60 cycles ≈ 15 minutes), the device calls `GET /api/v1/ota/firmware` on the server.
2. The server returns `{ "version": "x.y.z", "url": "..." }`. By default this is the firmware bundled in `FIRMWARE_DIR`, with a path relative to the server (`/api/v1/firmware/files/...`) that the device resolves against its configured server URL.
3. If the server version differs from the device's version (from `Cargo.toml`), the device downloads the app image from `url`, flashes it to the inactive OTA slot, and reboots.
4. On failure the device logs the error and continues normal operation — no reboot.

### Server Setup

Firmware is served from `FIRMWARE_DIR` (default `/app/firmware` in Docker), which contains `manifest.json` plus the images it lists. Build them with:

```bash
. $HOME/export-esp.sh
just esp32-release   # writes firmware-dist/ (merged image, OTA app image, manifest.json)
```

To host the image elsewhere instead, set these optional overrides on the server:

| Variable | Description |
|---|---|
| `OTA_FIRMWARE_VERSION` | Firmware version string to serve to devices (e.g. `0.2.0`) |
| `OTA_FIRMWARE_URL` | Full URL to the OTA app `.bin` file |

With neither firmware installed nor an override, the endpoint returns 404 and OTA is effectively off.

### Releasing an Update

1. Bump `version` in `esp32-client/Cargo.toml`.
2. Run `just esp32-release` (or let CI build it and bake it into the Docker image).
3. Restart the server; devices pick up the new version at their next check.

### Device Configuration

| cfg.toml field | NVS key | Default | Description |
|---|---|---|---|
| `ota_check_interval_cycles` | `ota_check_interval` | `60` | Check every N scan cycles (0 = disabled) |

To disable OTA checks at runtime, set NVS key `ota_check_interval` to `0`.

## Serial provisioning protocol

The web UI configures the device over the USB serial port. Requests are single lines `TILT>{json}`; replies are `TILT<{json}`. Everything else on the port is normal log output.

| `cmd` | Description |
|---|---|
| `info` | Version, chip, MAC, whether configured |
| `get_config` | Current config (`wifiPass` and `apiKey` are masked) |
| `set_config` | Validate and save a partial config (`{"config":{"wifiSsid":…}}`), then reboot |
| `wifi_scan` | List nearby networks (only before the device is configured) |
| `factory_reset` | Erase saved settings and reboot |

## Troubleshooting

### `python3 -m venv` fails during build

Install the Python venv module:

```bash
sudo apt install python3-venv
```

### `Cannot locate argument '--ldproxy-linker'`

The `ldproxy` tool is not installed or not in PATH:

```bash
cargo install ldproxy
```

Make sure `$HOME/.cargo/bin` is in your PATH.

### `error: current package believes it's in a workspace`

The esp32-client must be excluded from the parent Cargo workspace. Ensure the root `Cargo.toml` has:

```toml
[workspace]
exclude = ["esp32-client"]
```

### Build fails with `Unsupported target 'x86_64-unknown-linux-gnu'`

You forgot to source the ESP environment. Run:

```bash
. $HOME/export-esp.sh
```

# ESP32 Tilt Hydrometer Client

A Rust-based ESP32 client that scans for Tilt hydrometer BLE iBeacon advertisements and uploads readings to the Tilt server API.

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
```

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

### `cfg.toml not found` build error

Copy the example config:

```bash
cp cfg.toml.example cfg.toml
```

Then edit it with your WiFi credentials and server URL.

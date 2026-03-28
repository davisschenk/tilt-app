# Rust on ESP32 — Best Practices Guide

Authoritative reference for AI agents and contributors writing embedded Rust firmware
for the ESP32 family using the `esp-idf-svc` / `esp32-nimble` / `embuild` ecosystem.

---

## Table of Contents

1. [Toolchain Setup](#1-toolchain-setup)
2. [Project Structure & Build System](#2-project-structure--build-system)
3. [Dependency Management](#3-dependency-management)
4. [Error Handling](#4-error-handling)
5. [Memory & Heap Management](#5-memory--heap-management)
6. [Configuration: Compile-time vs Runtime](#6-configuration-compile-time-vs-runtime)
7. [WiFi — BlockingWifi Pattern](#7-wifi--blockingwifi-pattern)
8. [HTTP Client](#8-http-client)
9. [BLE Scanning with esp32-nimble](#9-ble-scanning-with-esp32-nimble)
10. [Non-Volatile Storage (NVS)](#10-non-volatile-storage-nvs)
11. [Task Watchdog Timer (TWDT)](#11-task-watchdog-timer-twdt)
12. [OTA Firmware Updates](#12-ota-firmware-updates)
13. [WiFi + BLE Coexistence](#13-wifi--ble-coexistence)
14. [SNTP Time Synchronization](#14-sntp-time-synchronization)
15. [FreeRTOS Threads from Rust std](#15-freertos-threads-from-rust-std)
16. [Async on ESP-IDF: block_on](#16-async-on-esp-idf-block_on)
17. [Logging](#17-logging)
18. [sdkconfig.defaults — Key Settings](#18-sdkconfigdefaults--key-settings)
19. [Partition Tables](#19-partition-tables)
20. [Code Style & Formatting](#20-code-style--formatting)
21. [Testing Strategy](#21-testing-strategy)
22. [Common Pitfalls](#22-common-pitfalls)

---

## 1. Toolchain Setup

ESP32 (Xtensa architecture) requires a custom LLVM backend not present in the standard
Rust toolchain.

### Install

```sh
cargo install espup
espup install
```

This installs the `esp` toolchain channel, which includes the Xtensa LLVM backend and
standard library pre-compiled for `xtensa-esp32-espidf`.

### Activate the Environment

Every shell session that will build or flash must source the export script:

```sh
. ~/export-esp.sh
```

Add this to your shell profile for convenience.

### `rust-toolchain.toml`

Pin the toolchain in the crate root so every contributor and CI uses the same version:

```toml
[toolchain]
channel = "esp"
```

### Cargo configuration

`.cargo/config.toml` sets the build target and runner so `cargo run` flashes directly:

```toml
[build]
target = "xtensa-esp32-espidf"

[target.xtensa-esp32-espidf]
runner = "espflash flash --monitor"

[unstable]
build-std = ["std", "panic_abort"]
```

### ESP32-C3 / ESP32-S3 / RISC-V Targets

For RISC-V ESP32 variants (C3, C6, H2) the standard Rust toolchain works with the
`riscv32imc-esp-espidf` target — no `espup` required for those targets. The patterns
in this guide apply to both.

---

## 2. Project Structure & Build System

### Separate from Cargo Workspaces

ESP32 crates that use `esp-idf-sys` **cannot** be members of a normal Cargo workspace
alongside standard `x86_64` crates. The ESP-IDF build system generates its own CMake
artifacts and expects to own the build directory. Keep the ESP32 crate as a standalone
project.

### Mandatory `build.rs`

Every ESP32 crate **must** call `embuild::espidf::sysenv::output()` in `build.rs`:

```rust
fn main() {
    embuild::espidf::sysenv::output();
}
```

This propagates ESP-IDF environment variables (IDF component paths, version, linker
flags) into Cargo's build graph. Without it the linker cannot find ESP-IDF symbols and
the build fails with mysterious unresolved references.

Add `embuild` as a build dependency:

```sh
cargo add --build embuild
```

### Cargo.toml Profile Settings

```toml
[profile.release]
opt-level = "s"    # optimize for size — flash is the bottleneck

[profile.dev]
opt-level = "z"    # aggressive size optimization even in dev
debug = true
```

Embedded targets have limited flash. Always optimize for size, not speed.

### `[[bin]]` Section

```toml
[[bin]]
name = "my-esp32-firmware"
harness = false    # required — the default test harness cannot run on ESP-IDF
```

`harness = false` is not optional. The standard Rust test harness calls OS APIs that
do not exist on ESP-IDF.

---

## 3. Dependency Management

### The Essential Stack

| Crate | Role |
|---|---|
| `esp-idf-svc` | Safe Rust wrappers for all ESP-IDF services. Re-exports `esp-idf-hal` and `esp-idf-sys` — depend only on this one. |
| `esp32-nimble` | NimBLE BLE stack wrapper. Use for BLE scanning/advertising/GATT. |
| `anyhow` | Ergonomic error propagation. Essential for `no_std`-adjacent embedded code. |
| `serde` + `serde_json` | Serialization for HTTP payloads. Both work on ESP-IDF `std`. |
| `heapless` | Stack-allocated collections (`String<N>`, `Vec<N>`). Required by some `esp-idf-svc` APIs. |
| `toml-cfg` | Compile-time configuration injection from a `cfg.toml` file. |
| `embuild` | Build dependency only — ESP-IDF environment propagation. |

### Adding Dependencies

Always use `cargo add` from within the ESP32 crate directory. Never manually edit
`Cargo.toml` to add a crate:

```sh
cargo add anyhow
cargo add serde --features derive
cargo add heapless
```

### `features = ["critical-section"]` on esp-idf-svc

```toml
esp-idf-svc = { version = "0.52", features = ["critical-section"] }
```

The `critical-section` feature provides the `critical-section` crate integration
required by some portable embedded crates. Include it by default.

---

## 4. Error Handling

### Use `anyhow::Result<T>` Everywhere

All fallible functions must return `anyhow::Result<T>`. Never use `unwrap()` or
`expect()` in production code paths:

```rust
// WRONG
let nvs = EspDefaultNvsPartition::take().unwrap();

// CORRECT
let nvs = EspDefaultNvsPartition::take()
    .context("Failed to take NVS partition")?;
```

### Add Context at Every `?` Call Site

Use `.context("...")` from `anyhow::Context` on every `?`. The message must identify
_what was being attempted_, not what failed — the underlying error already says what
failed:

```rust
use anyhow::Context;

wifi.start().context("Failed to start WiFi")?;
nvs.set_str("key", value).context("NVS write failed for key")?;
ota.initiate_update().context("OTA: failed to initiate update")?;
```

### `anyhow::bail!` for Early Returns

```rust
if cfg.scan_interval_secs < 5 {
    anyhow::bail!(
        "scan_interval_secs must be >= 5, got {}",
        cfg.scan_interval_secs
    );
}
```

### Handling Non-Fatal Errors in Loops

The main firmware loop must never terminate due to a recoverable error. Match on
results explicitly and continue:

```rust
loop {
    match scan_for_devices() {
        Ok(devices) => upload(&devices),
        Err(e) => {
            log::warn!("Scan failed: {:?}", e);
            attempt_recovery();
        }
    }
}
```

### `unsafe` Blocks

All `unsafe` blocks must have an accompanying comment explaining the safety invariant:

```rust
unsafe {
    // Safety: esp_task_wdt_reset() is safe to call from any task that has
    // previously subscribed via esp_task_wdt_add(). The main task subscribes
    // during init_watchdog() before this code path is reachable.
    esp_idf_svc::sys::esp_task_wdt_reset();
}
```

---

## 5. Memory & Heap Management

### Stack Size

The default ESP-IDF main task stack (3 KiB) is too small for Rust. Set at minimum
8 KiB in `sdkconfig.defaults`:

```
CONFIG_ESP_MAIN_TASK_STACK_SIZE=8192
```

If you experience stack overflows (manifesting as random crashes or `LoadProhibited`
panics), increase this value first.

### Avoid Large Stack Allocations

Do not allocate large buffers on the stack inside functions. Use heap-allocated types
(`Vec`, `String`) or pre-allocated fixed-size types from `heapless`:

```rust
// WRONG — 4 KiB on the stack inside a nested call chain
let buf = [0u8; 4096];

// CORRECT — heap allocated
let mut buf = vec![0u8; 4096];

// CORRECT — stack allocated but sized at compile time with heapless
let mut s: heapless::String<32> = heapless::String::new();
```

### Cap Response Body Reads

When reading HTTP responses, always cap the maximum bytes read to prevent unbounded
heap growth:

```rust
const MAX_RESPONSE_BYTES: usize = 1024;

let mut body = Vec::with_capacity(MAX_RESPONSE_BYTES);
let mut chunk = [0u8; 256];

loop {
    match conn.read(&mut chunk) {
        Ok(0) | Err(_) => break,
        Ok(n) => {
            let space = MAX_RESPONSE_BYTES.saturating_sub(body.len());
            if space == 0 { break; }
            body.extend_from_slice(&chunk[..n.min(space)]);
        }
    }
}
```

### Monitor Free Heap

Log free heap in periodic health reports to catch memory leaks early:

```rust
let free_heap = unsafe { esp_idf_svc::sys::esp_get_free_heap_size() };
log::info!("Free heap: {} bytes", free_heap);
```

A healthy long-running firmware should show a stable free heap across many cycles.
Consistently declining free heap indicates a leak.

---

## 6. Configuration: Compile-time vs Runtime

### Compile-time Configuration with `toml-cfg`

Sensitive values (WiFi credentials) and values that must be available before any
peripherals are initialized should be baked in at compile time using `toml-cfg`:

```toml
# cfg.toml (gitignored — contains credentials)
[my-crate]
wifi_ssid     = "MyNetwork"
wifi_password = "s3cr3t"
server_url    = "http://192.168.1.100:8000"
scan_interval = 15
```

```rust
// src/config.rs
#[toml_cfg::toml_config]
pub struct Config {
    #[default("")]
    wifi_ssid: &'static str,

    #[default("")]
    wifi_password: &'static str,

    #[default("http://localhost:8000")]
    server_url: &'static str,

    #[default(15)]
    scan_interval: u32,
}
```

`CONFIG` is a `const` of type `Config` generated by the macro.

**Guard against missing `cfg.toml` in `build.rs`:**

```rust
fn main() {
    if !std::path::Path::new("cfg.toml").exists() {
        panic!(
            "\n\nERROR: cfg.toml not found.\n\
             Copy cfg.toml.example to cfg.toml and fill in your credentials.\n"
        );
    }
    embuild::espidf::sysenv::output();
}
```

This produces a clear build-time error instead of flashing a binary with empty
credentials.

### Runtime Configuration with NVS

Use NVS for values that may change after deployment without a reflash:

```rust
use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault};

const NAMESPACE: &str = "my_app";

fn read_nvs_string(nvs: &EspNvs<NvsDefault>, key: &str) -> Option<String> {
    let mut buf = [0u8; 256];
    match nvs.get_str(key, &mut buf) {
        Ok(Some(s)) => {
            let trimmed = s.trim_end_matches('\0').to_string();
            if trimmed.is_empty() { None } else { Some(trimmed) }
        }
        _ => None,
    }
}

fn write_nvs_string(
    nvs: &mut EspNvs<NvsDefault>,
    key: &str,
    value: &str,
) -> anyhow::Result<()> {
    nvs.set_str(key, value)
        .map_err(|e| anyhow::anyhow!("NVS set_str('{}') failed: {:?}", key, e))
}
```

Open the NVS namespace with `read_write = true` to allow writes:

```rust
let nvs_part = EspDefaultNvsPartition::take()
    .context("Failed to take NVS partition")?;

let mut nvs = EspNvs::<NvsDefault>::new(nvs_part.clone(), NAMESPACE, true)
    .context("Failed to open NVS namespace")?;
```

### NVS Override Pattern

Load compile-time defaults first, then apply NVS overrides on top. This means the
device works out of the box with factory defaults and can be reconfigured at runtime:

```rust
let mut cfg = config::CONFIG;                  // compile-time defaults
config::apply_nvs_overrides(&mut cfg, &nvs);   // runtime overrides
config::validate(&cfg)?;                        // fail fast on bad config
```

---

## 7. WiFi — BlockingWifi Pattern

### Use `BlockingWifi`, Not `AsyncWifi`

On ESP-IDF, the standard Tokio async runtime is not available. Use `BlockingWifi`,
which provides synchronous `connect()`, `wait_netif_up()`, and `is_connected()`:

```rust
use esp_idf_svc::wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi};

let peripherals = Peripherals::take()?;
let sys_loop = EspSystemEventLoop::take()?;
let nvs = EspDefaultNvsPartition::take()?;

let esp_wifi = EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?;
let mut wifi = BlockingWifi::wrap(esp_wifi, sys_loop)?;
```

### Connection Sequence

```rust
use core::convert::TryInto;

let ssid: heapless::String<32> = ssid_str.try_into()
    .map_err(|_| anyhow::anyhow!("SSID too long (max 32 chars)"))?;

let password: heapless::String<64> = password_str.try_into()
    .map_err(|_| anyhow::anyhow!("Password too long (max 64 chars)"))?;

wifi.set_configuration(&Configuration::Client(ClientConfiguration {
    ssid,
    password,
    auth_method: AuthMethod::WPA2Personal,
    ..Default::default()
}))?;

wifi.start().context("Failed to start WiFi")?;
wifi.connect().context("Failed to connect to WiFi")?;
wifi.wait_netif_up().context("Timed out waiting for IP address")?;

let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
log::info!("Connected — IP: {}", ip_info.ip);
```

### Auth Method

Use `AuthMethod::WPA2Personal` for password-protected networks. Use `AuthMethod::None`
for open networks. Do not hard-code `AuthMethod::None` for all cases — it will silently
fail to associate to secured APs.

### Reconnection

`wifi.connect()` and `wifi.wait_netif_up()` block internally for up to ~30 seconds.
Suspend the Task Watchdog Timer before calling them (see [Section 11](#11-task-watchdog-timer-twdt)).

For reconnection after a drop, try a lightweight reconnect first; fall back to a full
stop/start/connect cycle if that fails:

```rust
fn ensure_connected(wifi: &mut BlockingWifi<EspWifi<'static>>) -> anyhow::Result<()> {
    if wifi.is_connected().unwrap_or(false) {
        return Ok(());
    }

    log::warn!("WiFi disconnected — reconnecting");

    if wifi.connect().is_err() {
        // Full restart on failure
        let _ = wifi.stop();
        wifi.start().context("WiFi restart failed")?;
        wifi.connect().context("WiFi reconnect failed after restart")?;
    }

    wifi.wait_netif_up().context("IP assignment timed out")?;
    Ok(())
}
```

---

## 8. HTTP Client

### EspHttpConnection

Use `EspHttpConnection` from `esp_idf_svc::http::client`:

```rust
use esp_idf_svc::http::client::{Configuration as HttpConfig, EspHttpConnection};
use esp_idf_svc::http::Method;

let config = HttpConfig {
    crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
    timeout: Some(std::time::Duration::from_secs(15)),
    ..Default::default()
};

let mut conn = EspHttpConnection::new(&config)
    .context("Failed to create HTTP connection")?;
```

### HTTPS: Always Set `crt_bundle_attach`

```rust
crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
```

This attaches the ESP-IDF bundled CA certificate store to the TLS context. It is
harmless for plain HTTP and essential for HTTPS. Always include it — you will likely
add HTTPS support later and forgetting this causes silent TLS verification failures.

### POST Request Pattern

You **must** call `initiate_response()` before reading status or the body. Forgetting
this panics at runtime:

```rust
let payload = serde_json::to_vec(&data)?;
let content_len = payload.len().to_string();

conn.initiate_request(
    Method::Post,
    &url,
    &[
        ("Content-Type", "application/json"),
        ("Content-Length", &content_len),
    ],
)?;

conn.write_all(&payload)?;
conn.initiate_response()?;     // must be called before conn.status()

let status = conn.status();
if !(200..300).contains(&(status as u32)) {
    anyhow::bail!("HTTP POST failed with status {}", status);
}
```

### GET Request Pattern

```rust
conn.initiate_request(Method::Get, &url, &[("Accept", "application/json")])?;
conn.initiate_response()?;

let status = conn.status();
if !(200..300).contains(&(status as u32)) {
    anyhow::bail!("HTTP GET {} failed: status {}", url, status);
}

// Read body (see Section 5 for capped read pattern)
```

### Set `Content-Length` for POST

The ESP-IDF HTTP client buffers the body internally. Always provide `Content-Length`
for POST/PUT requests. Without it, the client may use chunked transfer encoding, which
some HTTP servers reject.

### Request Timeouts

Always set a `timeout`. The default is no timeout, meaning a stalled connection will
block forever and eventually trigger the TWDT:

```rust
timeout: Some(std::time::Duration::from_secs(15)),
```

Use shorter timeouts for health-check-style requests (e.g., 10 s) and longer ones for
OTA downloads where the response is large (e.g., `None` for OTA, feed WDT manually).

---

## 9. BLE Scanning with esp32-nimble

### sdkconfig Requirements

These keys are required in `sdkconfig.defaults`:

```
CONFIG_BT_ENABLED=y
CONFIG_BT_BLUEDROID_ENABLED=n
CONFIG_BT_NIMBLE_ENABLED=y
```

Bluedroid and NimBLE are **mutually exclusive**. NimBLE is lighter weight; use it.

Optionally increase the NimBLE host task stack if you have deep call chains in the scan
callback:

```
CONFIG_BT_NIMBLE_HOST_TASK_STACK_SIZE=5120
```

### BLEDevice Initialization

`BLEDevice::take()` initializes the NimBLE host stack and returns a `&'static BLEDevice`.
Call it exactly once. Calling it again without first calling `BLEDevice::deinit()`
panics:

```rust
use esp32_nimble::BLEDevice;

let ble_device = BLEDevice::take();
```

### Passive Scanning

For observer-only use cases (e.g., iBeacon receivers, BLE advertisers), always use
**passive scanning**. Active scanning sends SCAN_REQ packets and wastes power:

```rust
use esp32_nimble::BLEScan;
use esp_idf_svc::hal::task::block_on;

block_on(async {
    let mut scan = BLEScan::new();
    scan.active_scan(false)         // passive — no SCAN_REQ sent
        .interval(100)              // 100 × 0.625 ms = 62.5 ms
        .window(99)                 // 99 × 0.625 ms = 61.875 ms
        .filter_duplicates(false);  // receive all advertisements

    scan.start(&ble_device, duration_ms, |device, data| {
        // process data
        None::<()>   // returning None continues the scan
    }).await?;

    Ok::<(), anyhow::Error>(())
})?;
```

### Scan Interval and Window

- `interval` and `window` are in units of 0.625 ms.
- `window` must be ≤ `interval`.
- Setting `window ≈ interval` approaches 100% duty cycle — maximum detection
  probability at the cost of higher current draw.
- A typical low-power setting: `interval(160)` / `window(16)` (100 ms / 10 ms, 10% duty).
- For always-on scanners where power is not a concern: `interval(100)` / `window(99)`.

### Accessing Manufacturer Data

```rust
scan.start(&ble_device, duration_ms, |device, data| {
    if let Some(mfg) = data.manufacture_data() {
        let company_id = mfg.company_identifier;  // u16, already decoded
        let payload    = mfg.payload;              // &[u8], after company ID bytes
        let rssi       = device.rssi();            // i8

        // filter by company ID before heavy parsing
        if company_id == 0x004C {
            // Apple advertisement — process payload
        }
    }
    None::<()>
})?;
```

Filter by `company_identifier` before doing any payload parsing. This avoids
unnecessary work for the thousands of non-relevant advertisements in a busy environment.

### Deduplication Strategy

`filter_duplicates(true)` tells the BLE controller to suppress repeat advertisements
from the same address during a scan window. Use `filter_duplicates(false)` when you
want to receive every broadcast and track the most recent reading per device:

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

let seen: Arc<Mutex<HashMap<DeviceKey, Reading>>> = Arc::new(Mutex::new(HashMap::new()));

// In callback:
if let Ok(mut map) = seen.lock() {
    map.insert(device_key, latest_reading);  // last-wins per device
}
```

After the scan completes, drain the map to get one reading per device.

### BLE Stack Recovery

If scanning returns an error, attempt recovery by deinitializing and reinitializing
the BLE stack:

```rust
fn attempt_ble_recovery(ble_device: &mut &'static BLEDevice) -> anyhow::Result<()> {
    log::warn!("Attempting BLE stack recovery");
    BLEDevice::deinit().context("BLE deinit failed")?;
    *ble_device = BLEDevice::take();
    log::info!("BLE stack recovered");
    Ok(())
}
```

After a configurable number of consecutive recovery failures, call
`esp_idf_svc::sys::esp_restart()` to perform a full device reboot. This is the correct
last resort for an unrecoverable BLE hardware fault.

---

## 10. Non-Volatile Storage (NVS)

### Opening a Namespace

```rust
use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault};

let nvs_part = EspDefaultNvsPartition::take()
    .context("Failed to take NVS partition")?;

// read_write = true allows both reads and writes
let mut nvs = EspNvs::<NvsDefault>::new(nvs_part.clone(), "my_ns", true)
    .context("Failed to open NVS namespace 'my_ns'")?;
```

### Reading

```rust
// Strings: provide a buffer; NVS writes into it
let mut buf = [0u8; 256];
let value: Option<&str> = nvs.get_str("my_key", &mut buf)
    .context("NVS read failed")?;

// Numeric types
let count: Option<u32> = nvs.get_u32("counter")
    .context("NVS read u32 failed")?;
```

### Writing

```rust
nvs.set_str("server_url", "http://10.0.0.1:8000")
    .context("NVS write 'server_url' failed")?;

nvs.set_u32("scan_interval", 30)
    .context("NVS write 'scan_interval' failed")?;
```

### Key Constraints

- **Max key length**: 15 characters (NVS hardware limit).
- **Max string value length**: partition-dependent, typically up to 4000 bytes, but
  keep values small (< 256 bytes) for robustness.
- **Namespace name**: max 15 characters.

### NVS and Static Strings

When NVS overrides a `&'static str` config field at runtime, use `Box::leak()` to
produce a `'static` reference from the heap-allocated `String`:

```rust
if let Some(val) = read_nvs_string(&nvs, "server_url") {
    cfg.server_url = Box::leak(val.into_boxed_str());
}
```

This is intentional: the configuration lives for the entire program lifetime, so
leaking is semantically correct.

---

## 11. Task Watchdog Timer (TWDT)

### Why It Matters

The TWDT is a hardware timer that reboots the device if the subscribed task does not
feed it within the timeout window. Essential for unattended firmware — a crash or
infinite loop results in a clean reboot rather than a silent hang.

### Configure Programmatically

Do not rely solely on `sdkconfig.defaults` values — they can be silently ignored if a
merged `sdkconfig` file already exists in the build tree. Configure the TWDT
programmatically at startup:

```rust
fn init_watchdog(timeout_ms: u32) -> anyhow::Result<()> {
    unsafe {
        let cfg = esp_idf_svc::sys::esp_task_wdt_config_t {
            timeout_ms,
            idle_core_mask: 0,      // do not monitor idle tasks
            trigger_panic: true,    // generate a panic dump on trigger
        };

        let ret = esp_idf_svc::sys::esp_task_wdt_reconfigure(&cfg);
        if ret != esp_idf_svc::sys::ESP_OK {
            anyhow::bail!("TWDT reconfigure failed: {}", ret);
        }

        // null_mut() means "the calling task" in the ESP-IDF TWDT API
        let ret = esp_idf_svc::sys::esp_task_wdt_add(core::ptr::null_mut());
        if ret != esp_idf_svc::sys::ESP_OK {
            anyhow::bail!("TWDT subscribe failed: {}", ret);
        }
    }
    log::info!("Watchdog initialized ({}s timeout)", timeout_ms / 1000);
    Ok(())
}
```

### Feeding

Call `esp_task_wdt_reset()` at least once per `timeout_ms` milliseconds:

```rust
fn feed_watchdog() {
    unsafe {
        esp_idf_svc::sys::esp_task_wdt_reset();
    }
}
```

Feed the watchdog at every checkpoint in the main loop — after BLE scan, after upload,
during long sleeps, during OTA downloads.

### Sleeping Without Starving the Watchdog

A plain `thread::sleep(duration)` blocks without feeding the TWDT. For intervals longer
than `~timeout/3`, break the sleep into chunks:

```rust
fn sleep_feeding_watchdog(duration: std::time::Duration, chunk_secs: u64) {
    let chunk = std::time::Duration::from_secs(chunk_secs);
    let mut remaining = duration;

    while remaining > std::time::Duration::ZERO {
        let nap = remaining.min(chunk);
        std::thread::sleep(nap);
        remaining = remaining.saturating_sub(nap);
        feed_watchdog();
    }
}
```

### Suspending During Blocking System Calls

Some ESP-IDF calls block for tens of seconds (WiFi connect, OTA flash erase) without
any opportunity to feed the TWDT. Temporarily unsubscribe the task before them:

```rust
fn suspend_watchdog() {
    unsafe {
        // null_mut() = current task
        esp_idf_svc::sys::esp_task_wdt_delete(core::ptr::null_mut());
    }
}

fn resume_watchdog() {
    unsafe {
        esp_idf_svc::sys::esp_task_wdt_add(core::ptr::null_mut());
        // Feed immediately so the full timeout window starts fresh
        esp_idf_svc::sys::esp_task_wdt_reset();
    }
}
```

**Always call `resume_watchdog()` in both success and error paths:**

```rust
suspend_watchdog();
let result = blocking_system_call().context("...");
resume_watchdog();   // called before result is propagated
result?;
```

### Disable Idle Task Monitoring When Using `block_on`

`block_on()` from `esp_idf_svc::hal::task` blocks the current task, starving the
FreeRTOS idle task. If the TWDT monitors idle tasks, this causes false triggers:

```
CONFIG_ESP_TASK_WDT_CHECK_IDLE_TASK_CPU0=n
CONFIG_ESP_TASK_WDT_CHECK_IDLE_TASK_CPU1=n
```

---

## 12. OTA Firmware Updates

### Dual-Slot OTA

ESP-IDF OTA requires a partition table with two app slots and an `otadata` partition:

```csv
# partitions.csv
nvs,      data, nvs,   0x9000,   0x8000,
phy_init, data, phy,   0x11000,  0x1000,
otadata,  data, ota,   0x12000,  0x2000,
ota_0,    app,  ota_0, 0x20000,  0x180000,
ota_1,    app,  ota_1, 0x1A0000, 0x180000,
```

Each app slot is 1.5 MiB in this example. Adjust sizes to fit your binary and flash
chip. Both slots must be the same size.

### Firmware Image Format

**Critical:** `cargo build` produces an ELF file. This is NOT the correct format for
OTA. You must convert it to an ESP32 app image using `espflash save-image`:

```sh
# Recommended (espflash 2.x):
espflash save-image --chip esp32 \
    target/xtensa-esp32-espidf/release/my-app \
    my-app.bin

# Alternative (esptool):
esptool.py --chip esp32 elf2image \
    --output my-app.bin \
    target/xtensa-esp32-espidf/release/my-app
```

The `.bin` file produced by `espflash save-image` is what the OTA client must download
and write to flash. Serving the raw ELF will cause `ESP_ERR_IMAGE_INVALID` during write.

### Firmware Metadata: `esp_app_desc!()`

Add this macro call near the top of `main.rs` to embed version metadata into the app
image. The OTA API uses it to read the running firmware's version:

```rust
use esp_idf_svc::sys::esp_app_desc;

esp_app_desc!();
```

This embeds the `version` field from `Cargo.toml` into the binary. Without it,
`ota.get_running_slot()?.firmware` returns `None`.

### Streaming Update Pattern

Stream the firmware binary directly from HTTP into flash. Never buffer the entire
binary in RAM. The correct completion method is `update.complete()` — not
`finish().activate()` (that is an older API that no longer exists in `esp-idf-svc 0.52`):

```rust
use esp_idf_svc::ota::EspOta;

fn perform_ota_update(url: &str) -> anyhow::Result<()> {
    let mut ota = EspOta::new().context("Failed to init OTA")?;
    let mut update = ota.initiate_update().context("Failed to begin OTA write")?;

    let result = download_and_write(url, &mut update);

    match result {
        Ok(_) => {
            // complete() marks the new slot valid and sets it as next boot target
            update.complete().context("OTA complete failed")?;
            Ok(())
        }
        Err(e) => {
            // abort() invalidates the partial write, leaving current firmware intact
            let _ = update.abort();
            Err(e)
        }
    }
}

const CHUNK_SIZE: usize = 8192;

fn download_and_write(
    url: &str,
    update: &mut esp_idf_svc::ota::EspOtaUpdate<'_>,
) -> anyhow::Result<usize> {
    let config = HttpConfig {
        buffer_size: Some(CHUNK_SIZE),
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        ..Default::default()
    };
    let mut conn = EspHttpConnection::new(&config)?;

    conn.initiate_request(Method::Get, url, &[("Accept", "application/octet-stream")])?;
    conn.initiate_response()?;

    if conn.status() != 200 {
        anyhow::bail!("OTA server returned HTTP {}", conn.status());
    }

    let mut buf = [0u8; CHUNK_SIZE];
    let mut total = 0usize;

    loop {
        let n = conn.read(&mut buf).context("OTA read error")?;
        if n == 0 { break; }

        update.write(&buf[..n]).context("OTA flash write error")?;
        total += n;

        // Feed watchdog every ~80 KiB — large binaries take tens of seconds
        if total % (CHUNK_SIZE * 10) == 0 {
            feed_watchdog();
            log::info!("OTA: {} KB written", total / 1024);
        }
    }

    Ok(total)
}
```

### After a Successful Update

Always reboot immediately after `complete()` to boot into the new firmware:

```rust
match perform_ota_update(&firmware_url) {
    Ok(()) => {
        log::info!("OTA complete — rebooting");
        esp_idf_svc::hal::reset::restart();
    }
    Err(e) => {
        log::error!("OTA failed: {:?}", e);
    }
}
```

Do not return to normal operation after `complete()` — the new firmware is not running
until after the reboot.

### OTA Rollback Support (Recommended for Production)

Enable rollback in `sdkconfig.defaults`:

```
CONFIG_BOOTLOADER_APP_ROLLBACK_ENABLE=y
```

With rollback enabled, if the device reboots without the new firmware explicitly marking
itself valid, the bootloader automatically reverts to the previous slot. Add a validity
check at startup:

```rust
fn check_and_mark_valid() -> anyhow::Result<()> {
    let mut ota = EspOta::new()?;
    let slot = ota.get_running_slot()?;

    if slot.state == esp_idf_svc::ota::SlotState::Factory {
        return Ok(());  // factory slot cannot be marked
    }

    if slot.state != esp_idf_svc::ota::SlotState::Valid {
        // Perform your health checks here (WiFi connect, API reachable, etc.)
        let is_healthy = true;
        if is_healthy {
            ota.mark_running_slot_valid()?;
        } else {
            ota.mark_running_slot_invalid_and_reboot();
        }
    }
    Ok(())
}
```

### Resetting `otadata` During Development

After a successful OTA update, the device boots from the second slot. When you
subsequently use `cargo run` / `espflash`, it flashes the first slot by default and
the device continues booting from the second (now stale) slot. Reset the `otadata`
partition to force a clean boot:

```toml
# .cargo/config.toml — add --erase-parts to runner
[target.xtensa-esp32-espidf]
runner = "espflash flash --monitor --erase-parts otadata"
```

Remove this flag in production; it erases OTA history on every flash.

---

## 13. WiFi + BLE Coexistence

The ESP32 has a single 2.4 GHz RF module shared between WiFi and Bluetooth. When both
are active simultaneously, the ESP-IDF uses **time-division multiplexing** managed by
a software coexistence arbitration module.

### Enable Software Coexistence

This sdkconfig key **must** be set when using WiFi and BLE simultaneously:

```
CONFIG_ESP_COEX_SW_COEXIST_ENABLE=y
```

Without it, WiFi and BLE fight over the RF hardware without coordination, causing
degraded throughput, missed BLE advertisements, and WiFi disconnections.

### How Time Slicing Works

The coexistence period is divided into time slices for WiFi and BLE. The proportions
depend on WiFi state:

- **WiFi IDLE**: BLE has full control.
- **WiFi CONNECTED**: ~50/50 split, period aligned to beacon interval (TBTT).
- **WiFi SCANNING**: WiFi gets a larger slice; BLE slice shrinks proportionally.
- **WiFi CONNECTING**: WiFi gets the largest slice.

This means **BLE scan duty cycle is effectively reduced when WiFi is connected**.
Factor this into your scan window / interval settings.

### BLE Scan Interruption

In coexistence mode, WiFi can preempt the BLE controller mid-scan-window and release
the RF before the scan window ends. Enable full-scan support so NimBLE can recapture
RF resources within the same scan window:

```
CONFIG_BTDM_CTRL_FULL_SCAN_SUPPORTED=y
```

### CPU Pinning for Performance

On dual-core ESP32 (not C3/S2), pin the BLE and WiFi tasks to separate cores to
minimize scheduling contention:

```
# Pin BLE controller + NimBLE host to Core 0
CONFIG_BTDM_CTRL_PINNED_TO_CORE_CHOICE=0
CONFIG_BT_NIMBLE_PINNED_TO_CORE_CHOICE=0

# Pin WiFi to Core 1
CONFIG_ESP_WIFI_TASK_CORE_ID=1
```

This is not required on single-core targets (ESP32-C3, ESP32-S2).

### Memory Reduction When Using Both

Running WiFi and BLE simultaneously uses significant RAM. If heap is tight, tune these
in `sdkconfig.defaults`:

```
# Dynamic BLE memory allocation (saves static RAM)
CONFIG_BT_BLE_DYNAMIC_ENV_MEMORY=y

# Reduce WiFi buffers
CONFIG_ESP_WIFI_STATIC_RX_BUFFER_NUM=4
CONFIG_ESP_WIFI_DYNAMIC_RX_BUFFER_NUM=8
CONFIG_ESP_WIFI_TX_BUFFER=DYNAMIC
CONFIG_ESP_WIFI_DYNAMIC_TX_BUFFER_NUM=16

# Reduce LwIP TCP buffers
CONFIG_LWIP_TCP_SND_BUF_DEFAULT=2880
CONFIG_LWIP_TCP_WND_DEFAULT=2880
```

Monitor free heap before and after enabling BLE to assess actual memory impact.

---

## 14. SNTP Time Synchronization

### Initialization

```rust
use esp_idf_svc::sntp::{EspSntp, SyncStatus};

// Must be called after WiFi is connected and an IP obtained
let _sntp = EspSntp::new_default().context("Failed to init SNTP")?;
```

### Lifetime: Keep the Handle Alive

**The `EspSntp` instance must be kept alive for the duration of the program.**
Dropping it stops the SNTP service and the system clock stops being updated:

```rust
// WRONG — sntp is dropped at end of block
{
    let sntp = EspSntp::new_default()?;
}  // SNTP stops here

// CORRECT — bind to a variable in the outer scope
let _sntp = EspSntp::new_default()?;  // lives until end of run()
```

### Waiting for Initial Sync

SNTP sync is asynchronous. Poll `get_sync_status()` before using `SystemTime::now()`
for timestamps that need to be accurate:

```rust
let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
while _sntp.get_sync_status() != SyncStatus::Completed {
    if std::time::Instant::now() > deadline {
        log::warn!("SNTP sync timed out — timestamps may be inaccurate");
        break;
    }
    std::thread::sleep(std::time::Duration::from_millis(500));
}
```

### Custom NTP Servers

```rust
use esp_idf_svc::sntp::{EspSntp, SntpConf, OperatingMode, SyncMode};

let sntp_conf = SntpConf {
    servers: ["pool.ntp.org", "time.cloudflare.com", "", ""],
    operating_mode: OperatingMode::Poll,
    sync_mode: SyncMode::Smooth,
};
let _sntp = EspSntp::new(&sntp_conf).context("Failed to init SNTP")?;
```

---

## 15. FreeRTOS Threads from Rust std

ESP-IDF backs `std::thread` with FreeRTOS tasks. All standard threading primitives
(`thread::spawn`, `Mutex`, `Arc`, `mpsc::channel`) work correctly.

### Always Set Stack Size for Spawned Threads

The default FreeRTOS task stack for `std::thread::spawn` is
`CONFIG_PTHREAD_TASK_STACK_SIZE_DEFAULT` (default 3 KiB), which is too small for most
Rust closures. Always use `thread::Builder` to set an explicit stack size:

```rust
std::thread::Builder::new()
    .stack_size(8 * 1024)   // 8 KiB — adjust based on what the closure does
    .spawn(move || {
        // thread body
    })
    .context("Failed to spawn thread")?;
```

Forget this and you get a hard-to-diagnose stack overflow, typically appearing as a
`LoadProhibited` exception inside the spawned closure.

### Channels for Inter-task Communication

Prefer `std::sync::mpsc::channel` over `Mutex<VecDeque>` for producer-consumer
patterns. It avoids blocking the producer when the consumer is slow:

```rust
use std::sync::mpsc;

let (tx, rx) = mpsc::channel::<Reading>();

std::thread::Builder::new()
    .stack_size(8 * 1024)
    .spawn(move || {
        for reading in rx {
            upload(reading);
        }
    })
    .context("Failed to spawn uploader thread")?;

// Producer:
tx.send(reading).ok();
```

### Thread vs Single-loop Architecture

For simple firmware (scan → upload → sleep), a **single-task main loop** is simpler
and avoids synchronization overhead. Use threads only when:

- Two genuinely concurrent operations must run simultaneously (e.g., BLE scan while
  serving an HTTP server).
- A blocking call (e.g., OTA download) must not stall the main loop.

---

## 16. Async on ESP-IDF: `block_on`

ESP-IDF does not support Tokio. Use `block_on` from `esp_idf_svc::hal::task` to drive
async code synchronously on the current FreeRTOS task:

```rust
use esp_idf_svc::hal::task::block_on;

block_on(async {
    some_async_function().await?;
    Ok::<(), anyhow::Error>(())
})?;
```

### Rules for `block_on`

- **Only use `block_on` for truly async APIs** (e.g., `esp32-nimble` BLE scan). Do not
  wrap synchronous code in `block_on` unnecessarily.
- **`block_on` is blocking** — it drives the async executor on the current task.
  The calling task is fully occupied until the future completes.
- **Do not nest `block_on` calls.** Calling `block_on` from within an async context
  driven by another `block_on` will deadlock or panic.
- **Disable idle task TWDT monitoring** when using `block_on`, as described in
  [Section 11](#11-task-watchdog-timer-twdt).

---

## 17. Logging

### Initialize at Startup

```rust
fn main() {
    esp_idf_svc::sys::link_patches();          // must be first
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Firmware starting");
}
```

`link_patches()` must be called before anything else. It links runtime patches from
`esp-idf-sys`. Forgetting it causes subtle runtime failures.

### Use the `log` Crate

```rust
log::error!("Fatal: {:?}", e);    // always visible
log::warn!("Retry attempt {}", n);
log::info!("WiFi connected: {}", ip);
log::debug!("Raw BLE payload: {:?}", bytes);  // only if LOG_MAXIMUM_LEVEL allows
```

Never use `println!` or `eprintln!` — they bypass the ESP-IDF logging system and can
cause issues with log buffering and UART contention.

### Log Level Configuration

In `sdkconfig.defaults`, set the log level for production builds:

```
CONFIG_LOG_DEFAULT_LEVEL_INFO=y
CONFIG_LOG_MAXIMUM_LEVEL_INFO=y
```

`LOG_MAXIMUM_LEVEL` is a compile-time cap. Debug logs compiled out at `INFO` level
have zero runtime overhead.

For development, change to `VERBOSE` or `DEBUG` temporarily.

### Sensitive Data

Never log credentials or API keys in plain text:

```rust
// WRONG
log::info!("Connecting with password: {}", password);

// CORRECT
log::info!("Connecting to '{}'", ssid);
log::debug!("API key present: {}", !api_key.is_empty());
```

---

## 18. sdkconfig.defaults — Key Settings

`sdkconfig.defaults` provides initial values that are merged into the project's
`sdkconfig` the first time the ESP-IDF build runs. Values already present in a
generated `sdkconfig` are not overwritten by subsequent changes to `sdkconfig.defaults`
(use `idf.py reconfigure` to force re-merge).

```
# Stack sizes — increase from defaults for Rust
CONFIG_ESP_MAIN_TASK_STACK_SIZE=8192
CONFIG_ESP_SYSTEM_EVENT_TASK_STACK_SIZE=4096
CONFIG_FREERTOS_IDLE_TASK_STACKSIZE=4096
CONFIG_PTHREAD_TASK_STACK_SIZE_DEFAULT=4096

# Flash size — must match your hardware
CONFIG_ESPTOOLPY_FLASHSIZE_4MB=y

# BLE — use NimBLE, not Bluedroid
CONFIG_BT_ENABLED=y
CONFIG_BT_BLUEDROID_ENABLED=n
CONFIG_BT_NIMBLE_ENABLED=y

# WiFi + BLE coexistence (required when using both)
CONFIG_ESP_COEX_SW_COEXIST_ENABLE=y
CONFIG_BTDM_CTRL_FULL_SCAN_SUPPORTED=y

# Watchdog — enable with panic on trigger
CONFIG_ESP_TASK_WDT_EN=y
CONFIG_ESP_TASK_WDT_PANIC=y
CONFIG_ESP_TASK_WDT_TIMEOUT_S=120

# Disable idle task monitoring (required when using block_on)
CONFIG_ESP_TASK_WDT_CHECK_IDLE_TASK_CPU0=n
CONFIG_ESP_TASK_WDT_CHECK_IDLE_TASK_CPU1=n

# Custom partition table
CONFIG_PARTITION_TABLE_CUSTOM=y
CONFIG_PARTITION_TABLE_CUSTOM_FILENAME="partitions.csv"

# OTA rollback (recommended for production)
CONFIG_BOOTLOADER_APP_ROLLBACK_ENABLE=y

# Log level — INFO for production
CONFIG_LOG_DEFAULT_LEVEL_INFO=y
CONFIG_LOG_MAXIMUM_LEVEL_INFO=y
```

### Do Not Edit Generated `sdkconfig`

The generated `sdkconfig` (in the build directory, not the crate root) is auto-generated
by the ESP-IDF CMake system. Only modify `sdkconfig.defaults` (committed) or use
`idf.py menuconfig` to make changes, which writes back to `sdkconfig.defaults`.

---

## 19. Partition Tables

### Custom Partition Table

A custom partition table is required for:
- **Dual OTA** (two app slots)
- **Larger NVS partition** than the default 24 KiB
- **SPIFFS or FAT** data partitions

Enable it in `sdkconfig.defaults`:

```
CONFIG_PARTITION_TABLE_CUSTOM=y
CONFIG_PARTITION_TABLE_CUSTOM_FILENAME="partitions.csv"
```

### Absolute Path Injection

The ESP-IDF CMake system resolves `CONFIG_PARTITION_TABLE_CUSTOM_FILENAME` relative to
its own build directory, not the crate root. Use `build.rs` to inject the absolute path:

```rust
fn main() {
    let abs = std::path::Path::new("partitions.csv")
        .canonicalize()
        .expect("partitions.csv not found");

    let out = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("sdkconfig.partitions");

    std::fs::write(
        &out,
        format!(
            "CONFIG_PARTITION_TABLE_CUSTOM=y\n\
             CONFIG_PARTITION_TABLE_CUSTOM_FILENAME=\"{}\"\n",
            abs.display()
        ),
    ).expect("Failed to write sdkconfig.partitions");

    println!("cargo:rerun-if-changed=partitions.csv");
    embuild::espidf::sysenv::output();
}
```

Add `sdkconfig.partitions` to `.gitignore` — it contains machine-specific absolute
paths and is regenerated on every build.

### Partition Size Constraints

- All partitions must fit within the flash size (`CONFIG_ESPTOOLPY_FLASHSIZE_4MB` = 4096 KiB).
- Partitions must be aligned to 4 KiB (0x1000) boundaries.
- App partitions must be aligned to 64 KiB (0x10000) boundaries.
- Both OTA slots must be the same size.

---

## 20. Code Style & Formatting

### Always Run `rustfmt`

```sh
cargo fmt
```

All code must be formatted with `rustfmt` using the Rust standard style (no custom
`rustfmt.toml` unless specifically required). Run `cargo fmt` before every commit.

### Import Groups

Organize imports in three groups, separated by a blank line, in this order:

```rust
// 1. Standard library
use std::collections::HashMap;
use std::time::{Duration, Instant};

// 2. External crates (alphabetical)
use anyhow::{Context, Result};
use esp_idf_svc::hal::task::block_on;
use serde::{Deserialize, Serialize};

// 3. Crate-internal
use crate::config::Config;
use crate::tilt::TiltReading;
```

### Naming

| Item | Convention | Example |
|---|---|---|
| Modules | `snake_case` | `ble`, `wifi`, `config` |
| Structs | `PascalCase` | `WifiManager`, `BleScanner` |
| Enums | `PascalCase` | `TiltColor`, `DeviceState` |
| Constants | `SCREAMING_SNAKE_CASE` | `MAX_RETRIES`, `CHUNK_SIZE` |
| Functions | `snake_case` | `connect_wifi`, `feed_watchdog` |
| Fields | `snake_case` | `scan_interval`, `server_url` |

### Module-level Doc Comments

Every source file must start with a `//!` module doc comment explaining:
1. What the module contains.
2. Any important invariants or constraints.

```rust
//! WiFi connection management.
//!
//! Manages the ESP32 WiFi peripheral in STA mode: initial connection,
//! monitoring, and reconnection with TWDT suspension around blocking calls.
```

### Whitespace and Readability

- Separate logical sections within a function with a blank line and an inline comment.
- Keep functions focused and short — if a function exceeds ~50 lines, consider splitting it.
- Use trailing commas in multi-line struct literals and match arms.

```rust
let config = HttpConfig {
    crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
    timeout: Some(Duration::from_secs(15)),
    ..Default::default()
};
```

---

## 21. Testing Strategy

### Host-Runnable Unit Tests

Most business logic (parsing, buffer behavior, backoff math, timestamp formatting)
can run on the host with `cargo test`. Place these tests in `#[cfg(test)]` blocks
within the module file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing_returns_none_for_short_input() {
        assert!(parse_ibeacon(&[0x02, 0x15]).is_none());
    }
}
```

Run them on the host:

```sh
# From the esp32-client directory, targeting the host (not ESP32)
cargo test --target x86_64-unknown-linux-gnu
```

Or use a test script that sets `CARGO_BUILD_TARGET` to the host target.

### What to Test on Host

- **Packet parsers** — input/output pairs for all valid and invalid inputs.
- **Buffer behavior** — capacity overflow, drain ordering, FIFO invariants.
- **Backoff math** — doubling, ceiling cap, overflow safety, reset behavior.
- **Timestamp formatting** — epoch, known dates, edge cases.
- **Config validation** — every validation rule with valid and invalid inputs.
- **UUID/enum round-trips** — serialize → deserialize, from_bytes → variant.

### What Cannot Be Tested on Host

- BLE scanning (requires NimBLE and hardware).
- WiFi connection (requires WiFi driver and hardware).
- NVS reads/writes (requires ESP-IDF NVS driver).
- OTA updates (requires flash hardware and OTA driver).
- Watchdog behavior (requires hardware timer).

For these, test manually on device and add integration test notes to the module doc.

### `harness = false` Implication

Because `harness = false` is set in `Cargo.toml`, you cannot use `cargo test` to run
tests when targeting the ESP32. Tests targeting the device must use a custom test runner
or be run manually.

---

## 22. Common Pitfalls

### Pitfall: Missing `link_patches()`

**Symptom**: Random crashes, missing symbols, or `abort()` called unexpectedly.  
**Fix**: `esp_idf_svc::sys::link_patches()` must be the very first line of `main()`.

---

### Pitfall: `unwrap()` on Peripheral Initialization

**Symptom**: Panic at boot if a peripheral is already taken.  
**Fix**: Use `?` with `.context()`. `Peripherals::take()` returns `Err` if called twice.

---

### Pitfall: TWDT Fires During Long Sleep

**Symptom**: Device reboots after exactly `watchdog_timeout_secs` seconds.  
**Fix**: Use `sleep_feeding_watchdog()` instead of `thread::sleep()` for any sleep
longer than `timeout / 3`.

---

### Pitfall: TWDT Fires on Idle Task During `block_on`

**Symptom**: TWDT panic with `IDLE0` or `IDLE1` in the trace during BLE scan.  
**Fix**: Set `CONFIG_ESP_TASK_WDT_CHECK_IDLE_TASK_CPU0=n` and `CPU1=n` in
`sdkconfig.defaults`.

---

### Pitfall: Stack Overflow During BLE Callback

**Symptom**: `LoadProhibited` or `StoreProhibited` exception with `PC` inside a BLE
callback. Address in stack region.  
**Fix**: Increase `CONFIG_BT_NIMBLE_HOST_TASK_STACK_SIZE` in `sdkconfig.defaults`.
Default is 4096–5120 bytes; increase to 7168 or 8192.

---

### Pitfall: WiFi `connect()` Blocks Forever

**Symptom**: Device hangs after "Connecting to WiFi..." log line.  
**Fix**: This is expected if the AP is unreachable. `BlockingWifi::connect()` will
time out after the driver's internal timeout (~10 s). The TWDT must be suspended during
this call. If the device hangs indefinitely, check that `suspend_watchdog()` was called.

---

### Pitfall: `initiate_response()` Not Called

**Symptom**: Panic at runtime in the HTTP module with a message about invalid state.  
**Fix**: Call `conn.initiate_response()?` after writing the request body and before
reading `conn.status()` or the response body. This is mandatory in the ESP-IDF HTTP
client API.

---

### Pitfall: `sdkconfig.defaults` Changes Ignored

**Symptom**: sdkconfig setting appears in `sdkconfig.defaults` but the build uses the
old value.  
**Fix**: The generated `sdkconfig` in the build directory takes precedence. To force
re-merge: `rm -rf build/` or run `idf.py reconfigure`. Configure programmatically at
runtime for values that must take effect reliably.

---

### Pitfall: Partition Table Path Resolution

**Symptom**: `ERROR: Partition table file not found: partitions.csv` during flash.  
**Fix**: Use `build.rs` to write the absolute path into `sdkconfig.partitions` as
described in [Section 16](#16-partition-tables). The CMake build system resolves the
path relative to its own working directory, not the crate root.

---

### Pitfall: NVS Key Too Long

**Symptom**: `ESP_ERR_NVS_KEY_TOO_LONG` at runtime when reading/writing NVS.  
**Fix**: NVS keys must be 15 characters or fewer (hardware limitation, not configurable).

---

### Pitfall: `Box::leak()` for NVS Override `&'static str`

**Symptom**: Borrow checker error when trying to assign a heap `String` to a
`&'static str` config field.  
**Fix**: `Box::leak(val.into_boxed_str())` is the correct pattern. It is intentional —
the configuration lives for the program lifetime so leaking is semantically sound.
Do not use this pattern for short-lived strings.

---

### Pitfall: `EspSntp` Dropped Too Early

**Symptom**: System clock stops updating after a period of time; `SystemTime::now()`
returns times that drift and never update.  
**Fix**: Bind the `EspSntp` handle to a variable in the outermost scope of your
program (e.g., `run()`) with `let _sntp = EspSntp::new_default()?;`. Dropping it
stops the background SNTP task.

---

### Pitfall: Flashing ELF Binary for OTA

**Symptom**: OTA write fails with `ESP_ERR_IMAGE_INVALID` immediately after the
first chunk is written.  
**Fix**: The `cargo build` ELF output is not a valid ESP32 app image. Convert it first:
```sh
espflash save-image --chip esp32 target/xtensa-esp32-espidf/release/my-app my-app.bin
```
Serve the `.bin` file, not the ELF, from your OTA server.

---

### Pitfall: Device Always Boots Second Slot After OTA (During Development)

**Symptom**: After a successful OTA update, `cargo run` flashes correctly but the
device still runs the old firmware from the OTA slot.  
**Fix**: Add `--erase-parts otadata` to the `espflash` runner in `.cargo/config.toml`
for development. This resets the boot slot selection. Remove it for production builds.

---

### Pitfall: WiFi Disconnects When BLE Scans Start

**Symptom**: WiFi drops or HTTP requests time out consistently when BLE scanning is
active. Works fine with either WiFi or BLE alone.  
**Fix**: Enable software coexistence: `CONFIG_ESP_COEX_SW_COEXIST_ENABLE=y`. Also
consider `CONFIG_BTDM_CTRL_FULL_SCAN_SUPPORTED=y` and CPU pinning (Section 13).

---

### Pitfall: Spawned Thread Stack Overflow

**Symptom**: `LoadProhibited` or `StoreProhibited` panic occurring inside a closure
passed to `std::thread::spawn`.  
**Fix**: Use `thread::Builder::new().stack_size(8192).spawn(...)` instead of the bare
`thread::spawn`. The default stack from `CONFIG_PTHREAD_TASK_STACK_SIZE_DEFAULT`
(often 3 KiB) is too small for typical Rust closures.

---

*End of guide.*

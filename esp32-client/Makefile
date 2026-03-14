# ESP32 Tilt Client — Build, Flash, and Monitor recipes
# Requires: ESP Rust toolchain (espup), ldproxy, espflash
# Source the environment first: . $HOME/export-esp.sh

BINARY := target/xtensa-esp32-espidf/release/esp32-client

# Build the firmware in release mode
.PHONY: build
build:
	cargo build --release

# Flash the firmware and open serial monitor
.PHONY: flash
flash: build
	espflash flash --monitor $(BINARY)

# Open serial monitor without reflashing
.PHONY: monitor
monitor:
	espflash monitor

# Remove build artifacts
.PHONY: clean
clean:
	cargo clean

# Run unit tests on host (tilt.rs, buffer.rs)
.PHONY: test
test:
	./test-host.sh

# Check compilation without building (faster feedback)
.PHONY: check
check:
	cargo check

# ESP32 Tilt Client — Build, Flash, and Monitor recipes
# Requires: ESP Rust toolchain (espup), ldproxy, espflash
# Source the environment first: . $HOME/export-esp.sh

BINARY := target/xtensa-esp32-espidf/release/esp32-client
BUILD_DIR := build

# Build the firmware in release mode
.PHONY: build
build:
	cargo build --release

# Build release binary and copy to build/esp32-client.bin for OTA hosting
.PHONY: release
release: build
	mkdir -p $(BUILD_DIR)
	cp $(BINARY) $(BUILD_DIR)/esp32-client.bin
	@echo "Firmware binary: $(BUILD_DIR)/esp32-client.bin"
	@ls -lh $(BUILD_DIR)/esp32-client.bin

# Flash the firmware and open serial monitor (full flash including partition table)
.PHONY: flash
flash: build
	espflash flash --monitor $(BINARY)

# Flash only the app binary to the next OTA slot (no partition table rewrite)
.PHONY: flash-ota
flash-ota: build
	espflash flash --partition-table partitions.csv $(BINARY)

# Open serial monitor without reflashing
.PHONY: monitor
monitor:
	espflash monitor

# Print the current firmware version from cfg.toml.example
.PHONY: version
version:
	@grep 'firmware_version' cfg.toml.example | head -1 | sed 's/.*= *"//' | sed 's/".*//'

# Remove build artifacts
.PHONY: clean
clean:
	cargo clean
	rm -rf $(BUILD_DIR)

# Run unit tests on host (tilt.rs, buffer.rs)
.PHONY: test
test:
	./test-host.sh

# Check compilation without building (faster feedback)
.PHONY: check
check:
	cargo check

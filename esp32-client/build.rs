fn main() {
    // Check that cfg.toml exists — toml-cfg silently uses defaults otherwise,
    // which would leave WiFi credentials empty and the device unable to connect.
    if !std::path::Path::new("cfg.toml").exists() {
        panic!(
            "\n\n\
            ========================================================\n\
            ERROR: cfg.toml not found in esp32-client/\n\n\
            Copy cfg.toml.example to cfg.toml and fill in your values:\n\
              cp cfg.toml.example cfg.toml\n\n\
            Required fields: wifi_ssid, wifi_password, server_url\n\
            See cfg.toml.example for all configurable options.\n\
            ========================================================\n"
        );
    }

    // Generate an sdkconfig fragment with the absolute path to partitions.csv.
    // ESP-IDF CMake resolves CONFIG_PARTITION_TABLE_CUSTOM_FILENAME relative to
    // its own build output directory, so we must use an absolute path.
    let partitions_src = std::path::Path::new("partitions.csv")
        .canonicalize()
        .ok();
    if let Some(abs_path) = partitions_src {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let fragment = std::path::Path::new(&manifest_dir).join("sdkconfig.partitions");
        std::fs::write(
            &fragment,
            format!(
                "CONFIG_PARTITION_TABLE_CUSTOM=y\nCONFIG_PARTITION_TABLE_CUSTOM_FILENAME=\"{}\"\n",
                abs_path.display()
            ),
        )
        .expect("Failed to write sdkconfig.partitions");
        println!("cargo:rerun-if-changed=partitions.csv");
    }

    embuild::espidf::sysenv::output();
}

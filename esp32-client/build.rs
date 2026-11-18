fn main() {
    println!("cargo:rerun-if-env-changed=TILT_RELEASE_VERSION");
    println!("cargo:rerun-if-env-changed=OTA_PUBLIC_KEY");
    // cfg.toml is optional: it supplies compile-time defaults for development.
    // Production images are generic and get their config over serial (web setup).
    if !std::path::Path::new("cfg.toml").exists() {
        println!(
            "cargo:warning=cfg.toml not found — building a generic image; configure the device via the web Device Setup page"
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

    // Rebuild whenever the user updates their credentials or configuration.
    println!("cargo:rerun-if-changed=cfg.toml");

    embuild::espidf::sysenv::output();
}

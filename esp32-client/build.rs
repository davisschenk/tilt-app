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

    embuild::espidf::sysenv::output();
}

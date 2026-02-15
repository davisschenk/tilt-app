mod scanner;
mod uploader;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "tilt-client", about = "Tilt Hydrometer BLE scanner and uploader")]
struct Args {
    #[arg(long, help = "Server URL to upload readings to")]
    server_url: String,

    #[arg(long, default_value_t = 15, help = "BLE scan interval in seconds")]
    scan_interval: u64,

    #[arg(long, default_value = "info", help = "Log level (trace, debug, info, warn, error)")]
    log_level: String,

    #[arg(long, default_value_t = 100, help = "Maximum number of readings to buffer locally")]
    buffer_size: usize,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_new(&args.log_level)
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!(
        server_url = %args.server_url,
        scan_interval = args.scan_interval,
        buffer_size = args.buffer_size,
        log_level = %args.log_level,
        "Starting Tilt Hydrometer BLE client"
    );
}

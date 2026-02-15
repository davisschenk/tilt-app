mod buffer;
mod scanner;
mod uploader;

use std::time::Duration;

use buffer::{Backoff, ReadingBuffer};
use clap::Parser;
use scanner::TiltScanner;
use uploader::Uploader;

#[derive(Parser, Debug)]
#[command(
    name = "tilt-client",
    about = "Tilt Hydrometer BLE scanner and uploader"
)]
struct Args {
    #[arg(long, help = "Server URL to upload readings to")]
    server_url: String,

    #[arg(long, default_value_t = 15, help = "BLE scan interval in seconds")]
    scan_interval: u64,

    #[arg(
        long,
        default_value = "info",
        help = "Log level (trace, debug, info, warn, error)"
    )]
    log_level: String,

    #[arg(
        long,
        default_value_t = 100,
        help = "Maximum number of readings to buffer locally"
    )]
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

    let scanner = match TiltScanner::new().await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to initialize BLE scanner: {e}");
            return;
        }
    };

    let uploader = Uploader::new(&args.server_url);
    let mut reading_buffer = ReadingBuffer::new(args.buffer_size);
    let mut backoff = Backoff::default();
    let scan_duration = Duration::from_secs(args.scan_interval);

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Received Ctrl+C, shutting down gracefully");
                break;
            }
            result = scanner.scan_once(scan_duration) => {
                match result {
                    Ok(mut readings) => {
                        tracing::info!(count = readings.len(), "Scan complete");

                        if !reading_buffer.is_empty() {
                            let buffered = reading_buffer.drain_all();
                            tracing::info!(buffered = buffered.len(), "Prepending buffered readings");
                            let mut all = buffered;
                            all.append(&mut readings);
                            readings = all;
                        }

                        if readings.is_empty() {
                            tracing::debug!("No readings to upload");
                            continue;
                        }

                        match uploader.upload_batch(&readings).await {
                            Ok(response) => {
                                tracing::info!(?response, "Upload successful");
                                backoff.reset();
                            }
                            Err(e) => {
                                tracing::warn!("Upload failed: {e}");
                                reading_buffer.push_batch(&readings);
                                let delay = backoff.next_delay();
                                tracing::info!(?delay, buffered = reading_buffer.len(), "Backing off");
                                tokio::time::sleep(delay).await;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Scan failed: {e}");
                    }
                }
            }
        }
    }

    tracing::info!("Tilt client shut down");
}

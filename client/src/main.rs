mod buffer;
mod scanner;
mod simulator;
mod uploader;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use buffer::{Backoff, ReadingBuffer};
use clap::Parser;
use scanner::TiltScanner;
use shared::TiltColor;
use simulator::TiltSimulator;
use uploader::Uploader;

#[derive(Parser, Debug)]
#[command(
    name = "tilt-client",
    about = "Tilt Hydrometer BLE scanner and uploader"
)]
struct Args {
    #[arg(long, help = "Server URL to upload readings to")]
    server_url: String,

    #[arg(long, default_value_t = 5, help = "BLE scan window in seconds (how long to listen per cycle)")]
    scan_interval: u64,

    #[arg(long, default_value_t = 60, help = "Minimum seconds between uploads per hydrometer color")]
    upload_interval: u64,

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

    #[arg(long, help = "API key for authenticating with the server (X-API-Key header)")]
    api_key: Option<String>,

    #[arg(
        long,
        default_value_t = false,
        help = "Run in simulate mode (no BLE hardware required)"
    )]
    simulate: bool,

    #[arg(
        long,
        default_value = "Red",
        help = "Comma-separated Tilt colors to simulate (e.g. 'Red,Blue')"
    )]
    sim_colors: String,

    #[arg(
        long,
        default_value_t = 1.055,
        help = "Simulated starting Original Gravity"
    )]
    sim_og: f64,

    #[arg(long, default_value_t = 1.012, help = "Simulated target Final Gravity")]
    sim_target_fg: f64,

    #[arg(
        long,
        default_value_t = 68.0,
        help = "Simulated base temperature in °F"
    )]
    sim_temp: f64,
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

    if args.simulate {
        tracing::info!(
            server_url = %args.server_url,
            scan_interval = args.scan_interval,
            upload_interval = args.upload_interval,
            buffer_size = args.buffer_size,
            sim_colors = %args.sim_colors,
            sim_og = args.sim_og,
            sim_target_fg = args.sim_target_fg,
            sim_temp = args.sim_temp,
            "Starting Tilt client in SIMULATE mode (no BLE hardware required)"
        );
    } else {
        tracing::info!(
            server_url = %args.server_url,
            scan_interval = args.scan_interval,
            upload_interval = args.upload_interval,
            buffer_size = args.buffer_size,
            log_level = %args.log_level,
            "Starting Tilt Hydrometer BLE client"
        );
    }

    let uploader = Uploader::new(&args.server_url, args.api_key.clone());
    let mut reading_buffer = ReadingBuffer::new(args.buffer_size);
    let mut backoff = Backoff::default();
    let scan_duration = Duration::from_secs(args.scan_interval);
    let upload_interval = Duration::from_secs(args.upload_interval);
    let mut last_uploaded: HashMap<TiltColor, Instant> = HashMap::new();

    if args.simulate {
        let colors: Vec<TiltColor> = args
            .sim_colors
            .split(',')
            .map(|s| {
                let s = s.trim();
                TiltColor::parse(s).unwrap_or_else(|| {
                    tracing::error!(color = s, "Invalid Tilt color name. Valid: Red, Green, Black, Purple, Orange, Blue, Yellow, Pink");
                    std::process::exit(1);
                })
            })
            .collect();

        let simulator = TiltSimulator::new(colors, args.sim_og, args.sim_target_fg, args.sim_temp);

        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    tracing::info!("Received Ctrl+C, shutting down gracefully");
                    break;
                }
                _ = tokio::time::sleep(scan_duration) => {
                    let mut readings = simulator.generate_readings();
                    for r in &readings {
                        tracing::info!(color = ?r.color, temp = format!("{:.1}", r.temperature_f), gravity = format!("{:.4}", r.gravity), rssi = ?r.rssi, "Simulated reading");
                    }
                    tracing::info!(count = readings.len(), "Simulated scan complete");

                    if !reading_buffer.is_empty() {
                        let buffered = reading_buffer.drain_all();
                        tracing::info!(buffered = buffered.len(), "Prepending buffered readings");
                        let mut all = buffered;
                        all.append(&mut readings);
                        readings = all;
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
            }
        }
    } else {
        let mut scanner = match TiltScanner::new().await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to initialize BLE scanner: {e}");
                return;
            }
        };

        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    tracing::info!("Received Ctrl+C, shutting down gracefully");
                    break;
                }
                result = scanner.next_batch(scan_duration) => {
                    match result {
                        Ok(all_readings) => {
                            tracing::debug!(count = all_readings.len(), "Scan complete");

                            let now = Instant::now();
                            let mut readings: Vec<_> = all_readings
                                .into_iter()
                                .filter(|r| {
                                    last_uploaded
                                        .get(&r.color)
                                        .map_or(true, |t| now.duration_since(*t) >= upload_interval)
                                })
                                .collect();

                            for r in &readings {
                                last_uploaded.insert(r.color, now);
                            }

                            if !reading_buffer.is_empty() {
                                let buffered = reading_buffer.drain_all();
                                tracing::info!(buffered = buffered.len(), "Prepending buffered readings");
                                let mut all = buffered;
                                all.append(&mut readings);
                                readings = all;
                            }

                            if readings.is_empty() {
                                tracing::debug!("No readings due (throttled)");
                                continue;
                            }

                            tracing::info!(count = readings.len(), "Uploading readings");

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
                            tracing::warn!("Scan failed: {e:#}");
                            tokio::time::sleep(Duration::from_secs(5)).await;
                        }
                    }
                }
            }
        }
    }

    tracing::info!("Tilt client shut down");
}

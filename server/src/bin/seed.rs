//! Seeder binary. Run via `cargo run -p server --bin seed -- --profile minimal`.

use anyhow::Result;
use clap::{Parser, ValueEnum};
use sea_orm::Database;
use server::seed;

#[derive(Parser, Debug)]
#[command(about = "Seed the tilt-app database with deterministic dev/test data")]
struct Args {
    #[arg(long, value_enum, default_value_t = Profile::Minimal)]
    profile: Profile,

    /// Wipe existing data before seeding (default: no-op if data exists)
    #[arg(long)]
    force: bool,

    /// Database URL (defaults to DATABASE_URL env)
    #[arg(long)]
    database_url: Option<String>,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Profile {
    Minimal,
    Full,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();
    let url = args
        .database_url
        .or_else(|| std::env::var("DATABASE_URL").ok())
        .ok_or_else(|| anyhow::anyhow!("DATABASE_URL not set and --database-url not given"))?;

    let db = Database::connect(&url).await?;

    let report = match args.profile {
        Profile::Minimal => seed::seed_minimal(&db, args.force).await?,
        Profile::Full => anyhow::bail!("profile 'full' is not implemented yet"),
    };

    if let Some(existing) = report.skipped_existing {
        tracing::info!(
            "seed: skipped — {existing} hydrometers already exist (use --force to wipe)"
        );
    } else {
        tracing::info!(
            "seed: inserted {} hydrometers, {} brews, {} readings, {} events, {} alert targets, {} alert rules",
            report.hydrometers,
            report.brews,
            report.readings,
            report.events,
            report.alert_targets,
            report.alert_rules
        );
    }

    Ok(())
}

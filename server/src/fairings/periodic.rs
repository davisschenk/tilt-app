//! Background database clean-ups that run on a fixed interval after liftoff.

use sea_orm::{DatabaseConnection, DbErr};
use std::future::Future;
use std::time::Duration;

/// Wait this long after liftoff before the first run, so a deploy or restart
/// does not start deleting while the server is still warming up.
const FIRST_RUN_DELAY: Duration = Duration::from_secs(60);

/// Run `job` every `every` on a background task. `job` returns how many rows it
/// removed; non-zero counts are logged at info, failures at warn (the next tick
/// retries).
pub fn spawn_every<F, Fut>(db: DatabaseConnection, name: &'static str, every: Duration, job: F)
where
    F: Fn(DatabaseConnection) -> Fut + Send + 'static,
    Fut: Future<Output = Result<u64, DbErr>> + Send + 'static,
{
    tokio::spawn(async move {
        let start = tokio::time::Instant::now() + FIRST_RUN_DELAY;
        let mut interval = tokio::time::interval_at(start, every);
        loop {
            interval.tick().await;
            match job(db.clone()).await {
                Ok(n) if n > 0 => tracing::info!(task = name, deleted = n, "Cleanup removed rows"),
                Ok(_) => {}
                Err(e) => tracing::warn!(task = name, error = %e, "Cleanup failed"),
            }
        }
    });
}

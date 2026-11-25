//! Background clean-ups that run on a fixed interval after liftoff.

use std::future::Future;
use std::time::Duration;

/// Wait this long after liftoff before the first run, so a deploy or restart
/// does not start deleting while the server is still warming up.
const FIRST_RUN_DELAY: Duration = Duration::from_secs(60);

/// Run `job` every `every` on a background task. `job` captures whatever it
/// needs and returns how many `what` it removed; non-zero counts are logged at
/// info as "removed {n} {what}", failures at warn (the next tick retries).
pub fn spawn_every<F, Fut, E>(what: &'static str, every: Duration, job: F)
where
    F: Fn() -> Fut + Send + 'static,
    Fut: Future<Output = Result<u64, E>> + Send + 'static,
    E: std::fmt::Display,
{
    tokio::spawn(async move {
        let start = tokio::time::Instant::now() + FIRST_RUN_DELAY;
        let mut interval = tokio::time::interval_at(start, every);
        loop {
            interval.tick().await;
            match job().await {
                Ok(n) if n > 0 => tracing::info!(task = what, removed = n, "removed {n} {what}"),
                Ok(_) => {}
                Err(e) => tracing::warn!(task = what, error = %e, "Periodic cleanup failed"),
            }
        }
    });
}

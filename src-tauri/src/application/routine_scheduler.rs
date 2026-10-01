use std::{sync::Arc, time::Duration};

use super::{routine_service::now, RoutineService};

/// Upper bound on a single wait, so clock changes or system sleep are noticed promptly.
pub const MAX_SCHEDULER_WAIT: Duration = Duration::from_secs(5 * 60);

/// Sleeps until the next routine is due, fires it, and repeats. The loop wakes early
/// whenever routines change. It reads local state only and never calls a model.
pub async fn run_routine_scheduler(service: Arc<RoutineService>) {
    let changes = service.changes();
    tracing::info!("routine scheduler started");
    loop {
        if let Err(error) = service.fire_due(now()) {
            tracing::error!(%error, "due routines could not be fired");
        }
        let wait = match service.next_due_at() {
            Ok(next) => wait_until(next, now()),
            Err(error) => {
                tracing::error!(%error, "next routine time could not be read");
                MAX_SCHEDULER_WAIT
            }
        };
        tokio::select! {
            () = tokio::time::sleep(wait) => {}
            () = changes.notified() => {}
        }
    }
}

fn wait_until(
    next: Option<chrono::DateTime<chrono::Utc>>,
    now: chrono::DateTime<chrono::Utc>,
) -> Duration {
    next.map_or(MAX_SCHEDULER_WAIT, |next| {
        (next - now)
            .to_std()
            .unwrap_or(Duration::ZERO)
            .min(MAX_SCHEDULER_WAIT)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(value: &str) -> chrono::DateTime<chrono::Utc> {
        value.parse().expect("timestamp")
    }

    #[test]
    fn waits_until_the_next_routine_within_the_cap() {
        let now = at("2026-01-10T10:00:00Z");
        assert_eq!(
            wait_until(Some(at("2026-01-10T10:01:30Z")), now),
            Duration::from_secs(90)
        );
        assert_eq!(
            wait_until(Some(at("2026-01-10T12:00:00Z")), now),
            MAX_SCHEDULER_WAIT
        );
        assert_eq!(wait_until(None, now), MAX_SCHEDULER_WAIT);
    }

    #[test]
    fn does_not_wait_for_overdue_routines() {
        let now = at("2026-01-10T10:00:00Z");
        assert_eq!(
            wait_until(Some(at("2026-01-10T09:00:00Z")), now),
            Duration::ZERO
        );
    }
}

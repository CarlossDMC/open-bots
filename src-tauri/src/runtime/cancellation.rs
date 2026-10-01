use tokio::sync::watch;

/// Requests cancellation of one running turn.
#[derive(Debug)]
pub struct Canceller(watch::Sender<bool>);

/// Observes cancellation of one running turn. Cheap to clone.
#[derive(Debug, Clone)]
pub struct CancellationSignal(watch::Receiver<bool>);

pub fn cancellation_pair() -> (Canceller, CancellationSignal) {
    let (sender, receiver) = watch::channel(false);
    (Canceller(sender), CancellationSignal(receiver))
}

impl Canceller {
    pub fn cancel(&self) {
        self.0.send_replace(true);
    }
}

impl CancellationSignal {
    /// A signal that is never cancelled.
    pub fn never() -> Self {
        let (_, signal) = cancellation_pair();
        signal
    }

    pub fn is_cancelled(&self) -> bool {
        *self.0.borrow()
    }

    /// Resolves once cancellation is requested. Never resolves for a dropped canceller
    /// that did not cancel.
    pub async fn cancelled(&mut self) {
        if self.0.wait_for(|cancelled| *cancelled).await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn resolves_after_cancel() {
        let (canceller, mut signal) = cancellation_pair();
        assert!(!signal.is_cancelled());
        canceller.cancel();
        tokio::time::timeout(Duration::from_secs(1), signal.cancelled())
            .await
            .expect("cancellation");
        assert!(signal.is_cancelled());
    }

    #[tokio::test]
    async fn never_resolves_without_cancel() {
        let mut signal = CancellationSignal::never();
        assert!(
            tokio::time::timeout(Duration::from_millis(20), signal.cancelled())
                .await
                .is_err()
        );
    }
}

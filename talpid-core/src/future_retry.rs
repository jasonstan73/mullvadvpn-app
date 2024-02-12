use rand::{distributions::OpenClosed01, Rng};
use std::{future::Future, ops::Deref, time::Duration};
use talpid_time::sleep;

/// Retries a future until it should stop as determined by the retry function, or when
/// the iterator returns `None`.
pub async fn retry_future<
    F: FnMut() -> O + 'static,
    R: FnMut(&T) -> bool + 'static,
    D: Iterator<Item = Duration> + 'static,
    O: Future<Output = T>,
    T,
>(
    mut factory: F,
    mut should_retry: R,
    mut delays: D,
) -> T {
    loop {
        let current_result = factory().await;
        if should_retry(&current_result) {
            if let Some(delay) = delays.next() {
                sleep(delay).await;
                continue;
            }
        }
        return current_result;
    }
}

/// Iterator that repeats the same interval, with an optional maximum no. of attempts.
pub struct ConstantInterval {
    interval: Duration,
    attempt: usize,
    max_attempts: Option<usize>,
}

impl ConstantInterval {
    /// Creates a `ConstantInterval` that repeats `interval`, at most `max_attempts` times.
    pub const fn new(interval: Duration, max_attempts: Option<usize>) -> ConstantInterval {
        ConstantInterval {
            interval,
            attempt: 0,
            max_attempts,
        }
    }
}

impl Iterator for ConstantInterval {
    type Item = Duration;

    fn next(&mut self) -> Option<Duration> {
        if let Some(max_attempts) = self.max_attempts {
            if self.attempt >= max_attempts {
                return None;
            }
        }
        self.attempt = self.attempt.saturating_add(1);
        Some(self.interval)
    }
}

/// Provides an exponential back-off timer to delay the next retry of a failed operation.
#[derive(Clone)]
pub struct ExponentialBackoff {
    next: Duration,
    factor: u32,
    max_delay: Option<Duration>,
}

impl ExponentialBackoff {
    /// Creates a `ExponentialBackoff` starting with the provided duration.
    ///
    /// All else staying the same, the first delay will be `initial` long, the second
    /// one will be `initial * factor`, third `initial * factor^2` and so on.
    pub const fn new(initial: Duration, factor: u32) -> ExponentialBackoff {
        ExponentialBackoff {
            next: initial,
            factor,
            max_delay: None,
        }
    }

    /// Set the maximum delay. By default, there is no maximum value set. The limit is
    /// `Duration::MAX`.
    pub const fn max_delay(mut self, duration: Option<Duration>) -> ExponentialBackoff {
        self.max_delay = duration;
        self
    }

    /// Returns the value of the delay and advances the next back-off delay.
    fn next_delay(&mut self) -> Duration {
        let next = self.next;

        if let Some(max_delay) = self.max_delay {
            if next > max_delay {
                return max_delay;
            }
        }

        self.next = next.saturating_mul(self.factor);

        next
    }
}

impl Iterator for ExponentialBackoff {
    type Item = Duration;
    fn next(&mut self) -> Option<Duration> {
        Some(self.next_delay())
    }
}

/// Adds jitter to a duration iterator
pub struct Jittered<I> {
    inner: I,
}

impl<I> Jittered<I> {
    /// Create an iterator of jittered durations
    pub const fn jitter(inner: I) -> Self {
        Self { inner }
    }
}

impl<I: Iterator<Item = Duration>> Iterator for Jittered<I> {
    type Item = Duration;
    fn next(&mut self) -> Option<Self::Item> {
        let next_value = self.inner.next()?;
        Some(jitter(next_value))
    }
}

impl<I> Deref for Jittered<I> {
    type Target = I;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Apply a jitter to a duration.
fn jitter(dur: Duration) -> Duration {
    apply_jitter(dur, rand::thread_rng().sample(OpenClosed01))
}

fn apply_jitter(duration: Duration, jitter: f64) -> Duration {
    let secs = (duration.as_secs() as f64) * jitter;
    let nanos = (duration.subsec_nanos() as f64) * jitter;
    let millis = (secs * 1000f64) + (nanos / 1000000f64);
    Duration::from_millis(millis as u64)
}

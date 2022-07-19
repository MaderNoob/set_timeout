use std::time::{Duration, Instant};

use futures::future::BoxFuture;

use crate::addr_of_boxed_future;

/// A unique identifier for a scheduled timeout. This implements ordering and equality traits
/// according to the `run_at` field, and if 2 identifiers have the same `run_at` they are compared
/// by the `boxed_future_addr` field.
#[derive(Debug, Clone)]
pub struct ScheduledTimeoutIdentifier {
    /// When should this scheduled timeout run.
    run_at: Instant,

    /// The memory address of the boxed future which should be executed by this timeout. This is
    /// used to differentiate different timeouts with the same `run_at`.
    /// This makes sure that there are no 2 timeouts with the same identifier, since no 2 timeouts
    /// can have the same boxed future, unless `unsafe` is used, because that would be mutable
    /// aliasing.
    boxed_future_addr: usize,
}

impl ScheduledTimeoutIdentifier {
    /// Creates a new scheduled timeout identifier given the point in time at which the timeout
    /// should be executed and its boxed future.
    pub fn new(run_at: Instant, boxed_future: &BoxFuture<'static, ()>) -> Self {
        Self {
            run_at,
            boxed_future_addr: addr_of_boxed_future(boxed_future),
        }
    }

    /// Returns the delay needed to wait for this scheduled timeout, if it is bigger than the
    /// minimum delay.
    pub fn get_delay(&self, min_sleep_duration: Duration) -> Option<Duration> {
        let now = Instant::now();
        if self.run_at < now + min_sleep_duration {
            // run_at < now + min_sleep_duration
            // implies
            // run_at - now < min_sleep_duration
            //
            // which means that the delay we'll need to wait for is smaller than min_sleep_duration, so
            // return `None`
            None
        } else {
            Some(self.run_at - now)
        }
    }
}

impl PartialEq for ScheduledTimeoutIdentifier {
    fn eq(&self, other: &Self) -> bool {
        self.run_at.eq(&other.run_at) && self.boxed_future_addr.eq(&other.boxed_future_addr)
    }
}

impl Eq for ScheduledTimeoutIdentifier {}

impl PartialOrd for ScheduledTimeoutIdentifier {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(
            self.run_at
                .cmp(&other.run_at)
                .then(self.boxed_future_addr.cmp(&other.boxed_future_addr)),
        )
    }
}

impl Ord for ScheduledTimeoutIdentifier {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.run_at
            .cmp(&other.run_at)
            .then(self.boxed_future_addr.cmp(&other.boxed_future_addr))
    }
}

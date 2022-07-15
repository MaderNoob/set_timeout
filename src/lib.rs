#![feature(map_first_last)]

use futures::future::BoxFuture;
use scheduler_task::{SchedulerTask, SchedulerTaskHandler, CancellationToken};
use std::{
    future::Future,
    ops::Deref,
    time::{Duration, Instant},
};

mod scheduled_timeout;
mod scheduler_task;

/// A timeout scheduler, used for running futures at some delay.
///
/// This scheduler will use a single `tokio` task no matter how many timeouts are scheduled.
#[derive(Debug)]
pub struct TimeoutScheduler {
    scheduler_task_handler: SchedulerTaskHandler,
}

impl TimeoutScheduler {
    /// Creates a new timeout scheduler.
    ///
    /// The `min_timeout_delay` parameter is the minimum delay to wait for a timeout. Any timeout
    /// with a delay smaller than this will be executed immediately.
    ///
    /// Note that a duration value greater than 0 for `min_timeout_delay`  means that the scheduled
    /// timeouts **may** execute earlier than their delay. They will be early by **at most** the duration
    /// of `min_timeout_delay`.
    ///
    /// A value of `None` for `min_timeout_delay` means that there is no minimum timeout delay,
    /// which means that the scheduler will wait for every timeout however small it is. This
    /// guarantees that the timeouts will never run before their delay has exceeded.
    pub fn new(min_timeout_delay: Option<Duration>) -> Self {
        Self {
            scheduler_task_handler: SchedulerTask::run(
                min_timeout_delay.unwrap_or_else(|| Duration::from_secs(0)),
            ),
        }
    }

    /// Executes the given future after the given delay has exceeded.
    /// The delay at which the future is actually executed might be bigger than the given `delay`.
    pub fn set_timeout<Fut>(&self, delay: Duration, f: Fut) -> CancellationToken
    where
        Fut: Future<Output = ()> + Send + 'static,
    {
        let boxed_future = Box::pin(f);
        self.scheduler_task_handler
            .schedule_timeout(Instant::now() + delay, boxed_future)
    }

    pub fn cancel_timeout(&self, cancellation_token: CancellationToken) {
        self.scheduler_task_handler
            .cancel_timeout(cancellation_token)
    }
}

/// Returns the address of this boxed future in memory.
fn addr_of_boxed_future<'a, T>(boxed_future: &BoxFuture<'a, T>) -> usize {
    boxed_future.deref() as *const (dyn Future<Output = T> + Send) as *const () as usize
}

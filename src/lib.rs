#![feature(map_first_last)]

use futures::future::BoxFuture;
use scheduler_task::{SchedulerTask, SchedulerTaskHandler};
use std::{
    future::Future,
    ops::Deref,
    time::{Duration, Instant},
};

pub use scheduler_task::CancellationToken;

mod scheduled_timeout;
mod scheduler_task;

/// A timeout scheduler, used for running futures at some delay.
///
/// This scheduler will use a single `tokio` task no matter how many timeouts are scheduled.
///
/// The scheduler may be wrapped in an [`Arc`] in order to share it across multiple tasks.
///
/// [`Arc`]: std::sync::Arc
#[derive(Debug)]
pub struct TimeoutScheduler {
    scheduler_task_handler: SchedulerTaskHandler,
}

impl TimeoutScheduler {
    /// Creates a new timeout scheduler.
    ///
    /// The `min_timeout_delay` parameter is the minimum delay to wait for a timeout. Any timeout
    /// which requires sleeping for a delay smaller than this will be executed immediately.
    ///
    /// Note that a duration value greater than 0 for `min_timeout_delay`  means that the scheduled
    /// timeouts **may** execute earlier than their delay. They will be early by **at most** the duration
    /// of `min_timeout_delay`. This may slightly increase the performance because it avoids
    /// unnecessary short sleeps.
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
    ///
    /// The delay at which the future is actually executed might be bigger than the given `delay`,
    /// and if `min_timeout_delay` was set, the delay might be even smaller than the given `delay`.
    ///
    /// This function returns a [`CancellationToken`], which allows cancelling this timeout using a
    /// call to [`TimeoutScheduler::cancel_timeout`]
    pub fn set_timeout<Fut>(&self, delay: Duration, f: Fut) -> CancellationToken
    where
        Fut: Future<Output = ()> + Send + 'static,
    {
        let boxed_future = Box::pin(f);
        self.scheduler_task_handler
            .schedule_timeout(Instant::now() + delay, boxed_future)
    }

    /// Cancels the timeout associated with the given cancellation token.
    /// If the timeout was already executed or already cancelled, this does nothing.
    pub fn cancel_timeout(&self, cancellation_token: CancellationToken) {
        self.scheduler_task_handler
            .cancel_timeout(cancellation_token)
    }
}

/// Returns the address of this boxed future in memory.
fn addr_of_boxed_future<'a, T>(boxed_future: &BoxFuture<'a, T>) -> usize {
    boxed_future.deref() as *const (dyn Future<Output = T> + Send) as *const () as usize
}

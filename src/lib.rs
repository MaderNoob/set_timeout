//! This crate allows executing futures after a certain delay, similar to how `setTimeout` works in
//! js. The scheduling of a future can be cancelled as long as the future wasn't already executed,
//! by using a `CancellationToken`.
//!
//! This crate uses a scheduler which allows running an unlimited number of delayed futures using
//! just a single `tokio` task, in contrast to many crates which spawn a new task for each set
//! timeout call. This uses less memory and should be more performant.
//!
//! Please note that this crate can only be used in the context of a tokio runtime.
//!
//! # Example
//!
//! ```rust
//! #[tokio::main]
//! async fn main() {
//!     let scheduler = TimeoutScheduler::new(None);
//!
//!     let start = Instant::now();
//!
//!     // schedule a future which will run after at least 1.234 seconds from now.
//!     scheduler.set_timeout(Duration::from_secs_f32(1.234), async move {
//!         let elapsed = start.elapsed();
//!
//!         assert!(elapsed.as_secs_f32() > 1.234);
//!
//!         println!("elapsed: {:?}", elapsed);
//!     });
//!
//!     // make sure that the main task doesn't end before the timeout is executed, because if the main
//!     // task returns the runtime stops running.
//!     tokio::time::sleep(Duration::from_secs(2)).await;
//! }
//! ```
//!
//! ## Sharing The Scheduler
//! The timeout scheduler can be shared between multiple tasks, by storing it in an [`Arc`], or by
//! storing it in a global variable using the `lazy_static` crate. For an example of this, check
//! out the example called `global_variable` in the examples directory.
//!
//! [`Arc`]: std::sync::Arc

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

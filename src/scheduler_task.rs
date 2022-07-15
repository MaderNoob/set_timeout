use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

use futures::future::BoxFuture;
use tokio::sync::mpsc;

use crate::scheduled_timeout::ScheduledTimeoutIdentifier;

/// A macro which is like the `?` operator for options types but for functions which return `()`.
macro_rules! some_or_return {
    ($e: expr) => {
        match $e {
            Some(v) => v,
            None => return,
        }
    };
}

pub struct SchedulerTask {
    /// A map of scheduled timeouts, which maps each scheduled timeout identifier to its
    /// corresponding boxed future. The timeouts are ordered according to the point in time at
    /// which they should be ran.
    scheduled_timeouts: BTreeMap<ScheduledTimeoutIdentifier, BoxFuture<'static, ()>>,

    /// A receiver for scheduler task requests.
    requests_channel_receiver: mpsc::UnboundedReceiver<SchedulerTaskRequest>,

    /// The current delay that the scheduler should wait. This is always the smallest delay for
    /// the timeout which will be first to finish.
    cur_delay: Option<Duration>,

    /// The minimum delay of a timeout. Any timeout with a delay smaller than this will be executed
    /// immediately.
    min_timeout_delay: Duration,
}

impl SchedulerTask {
    /// Creates a new scheduler task and runs it, returning a [`SchedulerTaskHandler`] for it.
    /// The `min_timeout_delay` parameter is the minimum delay to wait for a timeout. Any timeout
    /// with a delay smaller than this will be executed immediately.
    pub fn run(min_timeout_delay: Duration) -> SchedulerTaskHandler {
        // create a communication channel with the scheduler task
        let (sender, receiver) = mpsc::unbounded_channel();

        let mut scheduler_task = SchedulerTask {
            scheduled_timeouts: BTreeMap::new(),
            requests_channel_receiver: receiver,
            cur_delay: None,
            min_timeout_delay,
        };

        // spawn a task for the scheduler task
        tokio::spawn(async move {
            scheduler_task.main_loop().await;
        });

        SchedulerTaskHandler {
            schedule_channel_sender: sender,
        }
    }

    async fn main_loop(&mut self) {
        loop {
            match self.cur_delay {
                Some(cur_delay) => {
                    match tokio::time::timeout(cur_delay, self.requests_channel_receiver.recv())
                        .await
                    {
                        Ok(recv_result) => {
                            // we received another request before the timeout has occured, update
                            // the current delay according to the new request.
                            //
                            // using `some_or_return` because if we failed to receive it means that the
                            // sender was closed, which means that this task is no longer is used, so stop
                            // running the main loop.
                            let request = some_or_return!(recv_result);
                            self.handle_request(request).await;
                        }
                        Err(_) => {
                            // a timeout has occured, run the desired scheduled timeout.
                            // the timeout we are currently waiting for will always be the first
                            // one, since they are ordered by execution point in time.
                            //
                            // so basically we can just run the future of the first timeout.
                            let boxed_future =
                                self.scheduled_timeouts.first_entry().unwrap().remove();
                            boxed_future.await;

                            // after removing and running the task, update the cur delay.
                            self.update_cur_delay().await;
                        }
                    }
                }
                None => {
                    // if there is no current delay, wait for a new request.
                    //
                    // using `some_or_return` because if we failed to receive it means that the
                    // sender was closed, which means that this task is no longer is used, so stop
                    // running the main loop.
                    let request = some_or_return!(self.requests_channel_receiver.recv().await);
                    self.handle_request(request).await;
                }
            }
        }
    }

    /// handles the given schedule request and updates `[Self::cur_delay]` accoridingly.
    async fn handle_request(&mut self, request: SchedulerTaskRequest) {
        match request {
            SchedulerTaskRequest::ScheduleTimeout {
                identifier,
                boxed_future,
            } => {
                // add the scheduled timeout
                self.scheduled_timeouts.insert(identifier, boxed_future);

                // update the delay according to the added timeout
                self.update_cur_delay().await
            }
            SchedulerTaskRequest::CancelTimeout(cancellation_token) => {
                if self
                    .scheduled_timeouts
                    .remove(&cancellation_token.timeout_identifier)
                    .is_some()
                {
                    // update the delay according to the removed timeout
                    self.update_cur_delay().await
                }
            }
        }
    }

    /// Updates the current delay that the scheduler should wait to the delay of the smallest task.
    async fn update_cur_delay(&mut self) {
        // run in a loop because sometimes the delay of the timeout at the top of the heap will be
        // too small that we will just run it without waiting and we'll then have to examine the
        // next top of the heap.
        loop {
            // get the delay of the smallest timeout.
            let delay = match self.scheduled_timeouts.first_key_value() {
                Some((timeout_identifier, _boxed_future)) => {
                    timeout_identifier.get_delay(self.min_timeout_delay)
                }
                None => {
                    // if there are no timeouts to wait for, just set the delay to `None`.
                    self.cur_delay = None;
                    return;
                }
            };

            // check the delay of this timeout, if it is smaller than the minimum, run the
            // task and check the next one, otherwise
            match delay {
                Some(delay) => {
                    // if the delay of the smallest timeout is bigger than the minimum
                    // timeout, then we must wait for it, so set the current delay to it.
                    self.cur_delay = Some(delay);

                    // we found the smallest delay, so return.
                    return;
                }
                None => {
                    // if the delay of the smallest timeout is smaller than the minimum
                    // timeout, just run it and check the next smallest timeout.
                    let boxed_future = self.scheduled_timeouts.first_entry().unwrap().remove();

                    boxed_future.await;
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct SchedulerTaskHandler {
    /// A channel through which schedule requests can be sent to the scheduler task
    schedule_channel_sender: mpsc::UnboundedSender<SchedulerTaskRequest>,
}

impl SchedulerTaskHandler {
    /// Schedules a timeout to the scheduler task.
    pub fn schedule_timeout(
        &self,
        run_at: Instant,
        boxed_future: BoxFuture<'static, ()>,
    ) -> CancellationToken {
        let timeout_identifier = ScheduledTimeoutIdentifier::new(run_at, &boxed_future);

        let cancellation_token = CancellationToken { timeout_identifier: timeout_identifier.clone() };

        // this should never fail because the schedule task should never end.
        self.send_request(SchedulerTaskRequest::ScheduleTimeout {
            identifier: timeout_identifier,
            boxed_future,
        });

        cancellation_token
    }

    /// Cancels a timeout scheduled on the scheduler task
    pub fn cancel_timeout(&self, cancellation_token: CancellationToken) {
        self.send_request(SchedulerTaskRequest::CancelTimeout(cancellation_token))
    }

    /// Sends a request to the scheduler task, panicking if the task has dropped its receiver.
    fn send_request(&self, request: SchedulerTaskRequest) {
        if self.schedule_channel_sender.send(request).is_err() {
            panic!("scheduler task has stopped unexpectedly")
        }
    }
}

enum SchedulerTaskRequest {
    ScheduleTimeout {
        identifier: ScheduledTimeoutIdentifier,
        boxed_future: BoxFuture<'static, ()>,
    },
    CancelTimeout(CancellationToken),
}

/// A cancellation token which allows cancelling a timeout.
///
/// This token is returned from a call to [`TimeoutScheduler::set_timeout`].
///
/// [`TimeoutScheduler::set_timeout`]: crate::TimeoutScheduler::set_timeout
#[derive(Debug, Clone)]
pub struct CancellationToken {
    timeout_identifier: ScheduledTimeoutIdentifier,
}

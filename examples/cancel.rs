use std::time::Duration;

use set_timeout::TimeoutScheduler;

#[tokio::main]
async fn main() {
    let scheduler = TimeoutScheduler::new(None);

    // schedule a future which will run in 1 second from now. this timeout will be cancelled and
    // should not be executed, so we should panic if it does.
    let cancellation_token = scheduler.set_timeout(Duration::from_secs(1), async move {
        panic!("cancelled timeout was ran");
    });

    // schedule another future which will run in 1 second from now, but this one won't be
    // cancelled, so it should run.
    scheduler.set_timeout(Duration::from_secs(1), async move {
        println!("the task that wasn't cancelled was executed");
    });

    // wait a little and then cancel the first timeout.
    tokio::time::sleep(Duration::from_millis(990)).await;

    scheduler.cancel_timeout(cancellation_token);

    // make sure that the main task doesn't end before the timeout is executed, because if the main
    // task returns the runtime stops running.
    tokio::time::sleep(Duration::from_secs(1)).await;
}

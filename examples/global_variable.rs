use std::time::{Duration, Instant};

use rand::Rng;
use set_timeout::TimeoutScheduler;

// store the scheduler as a global variable so that we can use it anywhere.
lazy_static::lazy_static! {
    static ref TIMEOUT_SCHEDULER: TimeoutScheduler = TimeoutScheduler::new(None);
}

#[tokio::main]
async fn main() {
    for _ in 0..10 {
        // spawn new tasks to show that the scheduler can be used anywhere, not only on the main
        // task.
        tokio::spawn(async move {
            let start = Instant::now();
            let delay = Duration::from_millis(rand::thread_rng().gen_range(100..=1000));
            TIMEOUT_SCHEDULER.set_timeout(delay, async move {
                println!(
                    "desired delay: {:?}, actual delay: {:?}",
                    delay,
                    start.elapsed()
                );
            })
        });
    }

    // make sure that the main task doesn't end before the timeout is executed, because if the main
    // task returns the runtime stops running.
    tokio::time::sleep(Duration::from_secs(2)).await;
}

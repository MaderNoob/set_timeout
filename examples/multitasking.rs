use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use rand::Rng;
use set_timeout::TimeoutScheduler;

#[tokio::main]
async fn main() {
    // wrap the scheduler in an `Arc` so that we can share it between multiple tasks.
    let scheduler = Arc::new(TimeoutScheduler::new(None));

    for _ in 0..10 {
        let scheduler = Arc::clone(&scheduler);
        let start = Instant::now();
        tokio::spawn(async move {
            let delay = Duration::from_millis(rand::thread_rng().gen_range(100..=1000));
            scheduler.set_timeout(delay, async move {
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

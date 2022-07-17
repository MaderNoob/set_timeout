use std::time::{Duration, Instant};

use rand::Rng;
use set_timeout::TimeoutScheduler;

#[tokio::main]
async fn main() {
    const MIN_TIMEOUT_DELAY: Duration = Duration::from_millis(30);

    // create a scheduler with a non-empty `min_timeout_delay`.
    let scheduler = TimeoutScheduler::new(Some(MIN_TIMEOUT_DELAY));

    let start = Instant::now();

    // execute a future with a delay smaller than the minimum delay which we gave to the scheduler,
    // and make sure it executes immediately.
    scheduler.set_timeout(Duration::from_millis(20), async move {
        let elapsed = start.elapsed();

        // since the future had a delay smaller than the minimum delay, we expect it to run almost
        // immediately, or at least shorter than the requested delay
        assert!(elapsed < Duration::from_millis(20));

        println!("elapsed: {:?}", elapsed);
    });

    // execute many tasks with random delays to show that even timeouts with delays longer than
    // `min_timeout_delay` might be executed earlier than expected, because sometimes the delay
    // that the scheduler has to wait between 2 timeouts is smaller than the `min_timeout_delay`,
    // even if the delay of the timeout itself is not.
    for _ in 0..20 {
        let delay = Duration::from_millis(rand::thread_rng().gen_range(300..=1000));
        let start = Instant::now();
        scheduler.set_timeout(delay, async move{
            let elapsed = start.elapsed();

            if elapsed < delay{
                println!(
                    "timeout executed before delay has exceeded, expected delay: {:?}, actual delay: {:?}",
                    delay,
                    elapsed
                );


                // note that the amount of time by which the timeout is executed early is at most
                // `min_timeout_delay`
                assert!(delay - elapsed <= MIN_TIMEOUT_DELAY);
            }
        });
    }

    // make sure that the main task doesn't end before the timeout is executed, because if the main
    // task returns the runtime stops running.
    tokio::time::sleep(Duration::from_secs(1)).await;
}

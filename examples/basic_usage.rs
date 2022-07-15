use std::time::{Duration, Instant};

use set_timeout::TimeoutScheduler;

#[tokio::main]
async fn main() {
    let scheduler = TimeoutScheduler::new(None);

    let start = Instant::now();

    // schedule a future which will run after at least 1.234 seconds from now.
    scheduler.set_timeout(Duration::from_secs_f32(1.234), async move{
        let elapsed = start.elapsed();

        assert!(elapsed.as_secs_f32() > 1.234);

        println!("elapsed: {:?}", elapsed);
    });

    // make sure that the main task doesn't end before the timeout is executed, because if the main
    // task returns the runtime stops running.
    tokio::time::sleep(Duration::from_secs(2)).await;
}

# set_timeout

This crate allows executing futures after a certain delay, similar to how `setTimeout` works in
js. The scheduling of a future can be cancelled as long as the future wasn't already executed,
by using a `CancellationToken`.

This crate uses a scheduler which allows running an unlimited number of delayed futures using
just a single `tokio` task, in contrast to many crates which spawn a new task for each set
timeout call. This uses less memory and should be more performant.

Please note that this crate can only be used in the context of a tokio runtime.

# Example

```rust
#[tokio::main]
async fn main() {
    let scheduler = TimeoutScheduler::new(None);

    let start = Instant::now();

    // schedule a future which will run after at least 1.234 seconds from now.
    scheduler.set_timeout(Duration::from_secs_f32(1.234), async move {
        let elapsed = start.elapsed();

        assert!(elapsed.as_secs_f32() > 1.234);

        println!("elapsed: {:?}", elapsed);
    });

    // make sure that the main task doesn't end before the timeout is executed, because if the main
    // task returns the runtime stops running.
    tokio::time::sleep(Duration::from_secs(2)).await;
}
```

## Usage Tip
You can schedule many timeouts on the scheduler, but you should avoid scheduling futures
which take a long time to execute, since such futures can block the scheduler from executing
other scheduled timeouts, and may cause other timeouts to execute at a big delay.

If you really need to schedule some future which takes a long time, consider scheduling a 
future which spawns a new task and then does all the heavy stuff.

## Sharing The Scheduler
The timeout scheduler can be shared between multiple tasks, by storing it in an [`Arc`], or by
storing it in a global variable using the `lazy_static` crate. For an example of this, check
out the example called `global_variable` and the example called `multitasking` in the examples directory.

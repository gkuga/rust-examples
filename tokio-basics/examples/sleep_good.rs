// Sleeping the async way.
//
// tokio::time::sleep makes the Executor park the task and free the worker
// thread, so other tasks can run concurrently. A small difference in code,
// but a big one in behavior.

use std::time;

#[tokio::main]
async fn main() {
    // wait for completion with join
    tokio::join!(async move {
        // sleep 10 seconds
        let ten_secs = time::Duration::from_secs(10);
        tokio::time::sleep(ten_secs).await; // Tokio's sleep
    });
}

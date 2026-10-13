// Bad example: calling a blocking sleep inside async code.
//
// std::thread::sleep blocks the worker thread for 10 seconds, so no other
// async task can run on that thread in the meantime. See sleep_good.rs.

use std::{thread, time}; // standard thread module

#[tokio::main]
async fn main() {
    // wait for completion with join
    tokio::join!(async move {
        // join! waits for several tasks to finish
        // sleep 10 seconds with the standard thread function
        let ten_secs = time::Duration::from_secs(10);
        thread::sleep(ten_secs);
    });
}

// std::sync::Mutex in async code.
//
// If a task only locks, updates, and unlocks (no `.await` while holding the
// lock), std::sync::Mutex is fine and faster than an async Mutex.

use std::sync::{Arc, Mutex};

const NUM_TASKS: usize = 4; // number of tasks
const NUM_LOOP: usize = 100000; // number of loops

#[tokio::main]
async fn main() -> Result<(), tokio::task::JoinError> {
    let val = Arc::new(Mutex::new(0)); // shared by all tasks
    let mut v = Vec::new();
    for _ in 0..NUM_TASKS {
        let n = val.clone();
        let t = tokio::spawn(async move {
            // spawn NUM_TASKS tasks
            for _ in 0..NUM_LOOP {
                let mut n0 = n.lock().unwrap();
                *n0 += 1; // each task locks and increments
            }
        });

        v.push(t);
    }

    for i in v {
        i.await?;
    }

    println!(
        "COUNT = {} (expected = {})",
        *val.lock().unwrap(),
        NUM_LOOP * NUM_TASKS
    );
    Ok(())
}

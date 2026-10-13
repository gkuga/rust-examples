// tokio::sync::Mutex: needed when holding a lock across `.await`.
//
// lock_sleep holds the lock while sleeping. With std::sync::Mutex this could
// deadlock: lock_sleep is parked holding the lock, and the lock_only tasks on
// the worker threads block in lock() forever. In practice it does not even
// compile with tokio::spawn, because std's MutexGuard is not Send, and the
// Future of lock_sleep has to keep the guard across `.await`.

use std::{sync::Arc, time};
use tokio::sync::Mutex;

const NUM_TASKS: usize = 8;

// a task that only locks and increments
async fn lock_only(v: Arc<Mutex<u64>>) {
    let mut n = v.lock().await;
    *n += 1;
}

// a task that awaits while holding the lock
async fn lock_sleep(v: Arc<Mutex<u64>>) {
    let mut n = v.lock().await;
    let ten_secs = time::Duration::from_secs(10);
    tokio::time::sleep(ten_secs).await; // the problem: `.await` while holding the lock
    *n += 1;
}

#[tokio::main]
async fn main() -> Result<(), tokio::task::JoinError> {
    let val = Arc::new(Mutex::new(0));
    let mut v = Vec::new();

    // spawn the lock_sleep task
    let t = tokio::spawn(lock_sleep(val.clone()));
    v.push(t);

    for _ in 0..NUM_TASKS {
        let n = val.clone();
        let t = tokio::spawn(lock_only(n)); // spawn lock_only tasks
        v.push(t);
    }

    for i in v {
        i.await?;
    }
    Ok(())
}

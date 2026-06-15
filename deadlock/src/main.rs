use std::sync::{Arc, RwLock};
use std::thread;

mod banker;

// Minimal example to understand the difference between with and without *
fn deref_example() {
    // val is Arc<RwLock<bool>> — two layers of wrapping:
    //   *val                 → RwLock<bool>          (unwrap Arc)
    //   val.read()           → RwLockReadGuard<bool>  (Arc's Deref calls RwLock::read() automatically)
    //   *val.read().unwrap() → bool                   (unwrap the Guard; * binds after . so this is *(val.read().unwrap()))
    let val = Arc::new(RwLock::new(true));

    // --- Step-by-step breakdown ---

    // Step 1: val.read() acquires the Read lock and returns LockResult<RwLockReadGuard<bool>>
    let lock_result = val.read();
    println!("lock_result type: LockResult<RwLockReadGuard<bool>>");

    // Step 2: .unwrap() extracts the RwLockReadGuard<bool> (Read lock is now held)
    let guard = lock_result.unwrap();
    println!("guard (via *guard): {}", *guard); // need * to read the inner bool

    // Step 3: * dereferences the guard to get the bool (bool is Copy, so it's copied)
    let flag: bool = *guard;
    println!("flag (bool copied out): {}", flag);

    // Note: guard is still alive after copying the bool (since bool is Copy, not moved)
    // We explicitly drop() here to demonstrate when the lock is actually released
    drop(guard);

    // --- One-liner equivalent ---
    // All three steps collapsed: Read lock acquired, bool copied, guard dropped immediately
    let flag2 = *val.read().unwrap(); // type: bool
    println!("flag2 (one-liner): {}", flag2);
}

// Example 4-1: RwLock that causes a deadlock
// Attempting to acquire a Write lock while holding a Read lock causes a deadlock
fn deadlock_example() {
    let val = Arc::new(RwLock::new(true));

    let t = thread::spawn(move || {
        let flag = val.read().unwrap(); // ❶ Acquire Read lock → guard is now bound to flag
        if *flag {
            // KEY POINT: guard's scope extends beyond the if block!
            // The if block ending does NOT release the lock.
            // guard is still alive here, so the Read lock is still held.
            *val.write().unwrap() = false; // ❷ Trying to acquire Write lock → but Read lock not released yet → Deadlock!
            println!("flag is true");
        }
        // ← guard's scope would end here, but we never reach this point
    });

    t.join().unwrap();
}

// Example 4-1 (fixed): RwLock without deadlock
// Copy the value immediately so the Read lock guard is dropped before acquiring the Write lock
fn deadlock_example2() {
    let val = Arc::new(RwLock::new(true));

    let t = thread::spawn(move || {
        let flag = *val.read().unwrap(); // ❶ Acquire Read lock, copy the value, then immediately drop the guard
        if flag {
            *val.write().unwrap() = false; // ❷ Read lock already released, so Write lock can be acquired safely
            println!("flag is true");
        }
    });

    t.join().unwrap();
}

// Example 4-2: RwLock that causes a deadlock (alternative way)
// Without explicit scoping, the Read lock guard remains in scope
fn deadlock_example3() {
    let val = Arc::new(RwLock::new(true));

    let t = thread::spawn(move || {
        let _flag = val.read().unwrap(); // ❶ Acquire Read lock (guard in scope)
        // guard is still alive here → Read lock still held
        *val.write().unwrap() = false; // ❷ Trying to acquire Write lock → Deadlock! (Read lock not released)
        println!("flag is true");
    });

    t.join().unwrap();
}

// Example 4-2 (fixed): Another way to avoid deadlock using explicit scoping
// Limit the Read lock's scope by dropping it in a separate block before acquiring Write lock
fn deadlock_solve_example3() {
    let val = Arc::new(RwLock::new(true));

    let t = thread::spawn(move || {
        // Scope-limited Read lock
        {
            let _flag = val.read().unwrap(); // ❶ Acquire Read lock in limited scope
            // Read lock released here when block ends
        }
        // Read lock is now released, so Write lock can be safely acquired
        *val.write().unwrap() = false; // ❷ Write lock acquired successfully, no deadlock
        println!("deadlock avoided (with explicit scoping)");
    });

    t.join().unwrap();
}

fn main() {
    println!("=== deref_example ===");
    deref_example();

    println!("\n=== deadlock_example2 ===");
    println!("(This version avoids the deadlock)");
    deadlock_example2();

    println!("\n=== deadlock_example3 ===");
    println!("(Another way to cause deadlock - no explicit scoping)");
    // Uncomment to see deadlock (program will hang):
    // deadlock_example3();
    println!("(Skipped to avoid hang)");

    println!("\n=== deadlock_solve_example3 ===");
    println!("(Avoided by explicit scoping)");
    deadlock_solve_example3();

    println!("\n=== banker::single_thread_example ===");
    banker::single_thread_example();
    println!("Done.");
}

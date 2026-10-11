use std::sync::atomic::{
    AtomicBool,
    AtomicUsize,
    Ordering,
};
use std::thread;

static DATA: AtomicUsize = AtomicUsize::new(0);
static READY: AtomicBool = AtomicBool::new(false);

const ITERATIONS: usize = 10_000;

// Thread A (writer side)
fn writer() {
    DATA.store(100, Ordering::Relaxed);
    READY.store(true, Ordering::Release);
}

// Thread B (reader side)
// Returns whether READY was observed as true.
fn reader() -> bool {
    if READY.load(Ordering::Acquire) {
        let value = DATA.load(Ordering::Relaxed);
        assert_eq!(value, 100);
        true
    } else {
        false
    }
}

fn main() {
    let mut ready_count = 0;

    for _ in 0..ITERATIONS {
        // Reset the shared state before each run.
        DATA.store(0, Ordering::Relaxed);
        READY.store(false, Ordering::Relaxed);

        let a = thread::spawn(writer);
        let b = thread::spawn(reader);
        a.join().unwrap();
        if b.join().unwrap() {
            ready_count += 1;
        }
    }

    println!("iterations:      {ITERATIONS}");
    println!("READY was true:  {ready_count}");
    println!("READY was false: {}", ITERATIONS - ready_count);
}

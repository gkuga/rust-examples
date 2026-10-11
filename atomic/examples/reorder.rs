// Message-passing litmus test: checks whether the reader can observe READY
// before DATA when the store/load on READY uses Relaxed instead of
// Release/Acquire.
//
// Usage:
//   cargo run --release --example reorder -- relaxed
//   cargo run --release --example reorder -- acqrel
use std::env;
use std::sync::atomic::{
    AtomicUsize,
    Ordering,
};
use std::thread;

static DATA: AtomicUsize = AtomicUsize::new(0);
static READY: AtomicUsize = AtomicUsize::new(0);

const ROUNDS: usize = 10_000_000;

fn main() {
    let mode = env::args().nth(1).unwrap_or_else(|| "relaxed".to_string());
    let (store_order, load_order) = match mode.as_str() {
        "relaxed" => (Ordering::Relaxed, Ordering::Relaxed),
        "acqrel" => (Ordering::Release, Ordering::Acquire),
        _ => panic!("mode must be `relaxed` or `acqrel`"),
    };

    // Thread A (writer side)
    // Writes the round number to DATA first, then publishes it via READY.
    let writer = thread::spawn(move || {
        for i in 1..=ROUNDS {
            DATA.store(i, Ordering::Relaxed);
            READY.store(i, store_order);
        }
    });

    // Thread B (reader side)
    // If READY shows round r, DATA must be at least r when ordering is
    // guaranteed. DATA < READY means the reader saw the writes out of order.
    let reader = thread::spawn(move || {
        let mut violations = 0;
        loop {
            let ready = READY.load(load_order);
            let data = DATA.load(Ordering::Relaxed);
            if data < ready {
                violations += 1;
            }
            if ready == ROUNDS {
                return violations;
            }
        }
    });

    writer.join().unwrap();
    let violations = reader.join().unwrap();

    println!("mode:       {mode}");
    println!("rounds:     {ROUNDS}");
    println!("violations: {violations}");
}

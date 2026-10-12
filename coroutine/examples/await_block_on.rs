// `await_hello.rs`, driven by an Executor (`futures::executor::block_on`)
// instead of calling `poll` by hand.
//
// `.await` can only be used inside `async fn` / `async` blocks. At the top level
// (`main`), something must receive `Pending` and call `poll` again: that is the
// Executor's job. Here `block_on` does it, replacing the three manual `poll`
// calls, `Waker::noop()`, and `pin!` (block_on pins the future internally).
//
// The one required change is in `YieldOnce`: it must call `wake_by_ref()` before
// returning `Pending`. `block_on` sleeps until woken, so without it the program
// would hang after the first poll. (`await_hello.rs` works without waking only
// because it polls by hand with a no-op Waker.)

use futures::executor::block_on;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

// A Future that returns Pending once, then Ready.
// Awaiting it suspends the async fn once, like Python's `yield`.
struct YieldOnce {
    yielded: bool,
}

impl Future for YieldOnce {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.yielded {
            Poll::Ready(())
        } else {
            self.yielded = true;
            // A Future that returns Pending must arrange to be woken.
            // We are ready right away, so ask the Executor to poll again now.
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

fn yield_now() -> YieldOnce {
    YieldOnce { yielded: false }
}

async fn hello() {
    print!("Hello, ");
    yield_now().await; // suspend here
    println!("World!");
    yield_now().await; // suspend here
}

fn main() {
    // block_on polls hello() until it returns Ready, sleeping on each Pending
    // until YieldOnce wakes it.
    block_on(hello());
}

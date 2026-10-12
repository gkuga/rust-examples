// The hand-written `Hello` Future from `noop_waker.rs`, driven by an Executor
// (`futures::executor::block_on`) instead of calling `poll` by hand.
//
// The hand-written state machine (`Hello::poll`) is kept as is. `Hello` is itself
// a Future, so it can be passed to `block_on` directly; no async block is needed.
//
// The one required change is in `Hello::poll`: each `Pending` arm now calls
// `wake_by_ref()` (the book's self-scheduling `Hello`). `block_on` sleeps until
// woken, so without it the program would hang after the first poll.
// (`noop_waker.rs` works without waking only because it polls by hand.)

use futures::executor::block_on;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

struct Hello {
    state: StateHello,
}

enum StateHello {
    HELLO,
    WORLD,
    END,
}

impl Hello {
    fn new() -> Self {
        Hello {
            state: StateHello::HELLO, // initial state
        }
    }
}

impl Future for Hello {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        match (*self).state {
            StateHello::HELLO => {
                print!("Hello, ");
                (*self).state = StateHello::WORLD;
                cx.waker().wake_by_ref(); // ask the Executor to poll again
                Poll::Pending
            }
            StateHello::WORLD => {
                println!("World!");
                (*self).state = StateHello::END;
                cx.waker().wake_by_ref(); // ask the Executor to poll again
                Poll::Pending
            }
            StateHello::END => Poll::Ready(()),
        }
    }
}

fn main() {
    // block_on polls Hello until it returns Ready, sleeping on each Pending
    // until Hello wakes it.
    block_on(Hello::new());
}

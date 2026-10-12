// The manual-poll "Hello, World!" from `src/main.rs`, without Arc, Task, Mutex,
// or the `futures` crate.
//
// Why `src/main.rs` (written as in the book) uses Arc:
//   - `poll` needs a `&mut Context`, and a `Context` is built from a `Waker`.
//   - The book builds the Waker with `futures::task::waker_ref`, which requires
//     an `&Arc<impl ArcWake>`. So `Task` and `Arc` are there only to obtain a
//     Waker, not because anything is shared. The Waker is never even used:
//     `wake_by_ref` does nothing, and `main` calls `poll` by hand.
//   - The book targets Rust 1.50, which had no ready-made no-op Waker.
//     `std::task::Waker::noop()` was stabilized in Rust 1.85.
//   - The book also uses the same shape to prepare for the next section
//     (`examples/scheduling.rs`), where `Arc<Task>` is really needed: the run
//     queue, the Executor, and the Waker all hold the same Task.
//
// With `Waker::noop()`, none of that is needed here.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

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

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        match (*self).state {
            StateHello::HELLO => {
                print!("Hello, ");
                (*self).state = StateHello::WORLD;
                Poll::Pending
            }
            StateHello::WORLD => {
                println!("World!");
                (*self).state = StateHello::END;
                Poll::Pending
            }
            StateHello::END => Poll::Ready(()),
        }
    }
}

fn main() {
    // A Waker that does nothing. We poll by hand, so nobody needs to be woken.
    let mut cx = Context::from_waker(Waker::noop());

    // No Box or Mutex: the Future lives in a local variable.
    // Hello is Unpin, so Pin::new can pin it safely.
    let mut hello = Hello::new();

    // Suspend and resume repeatedly.
    let _ = Pin::new(&mut hello).poll(&mut cx);
    let _ = Pin::new(&mut hello).poll(&mut cx);
    let _ = Pin::new(&mut hello).poll(&mut cx);
}

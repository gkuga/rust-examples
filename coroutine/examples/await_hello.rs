// `noop_waker.rs` rewritten with async/await.
//
// Instead of writing the state machine (`Hello` + `StateHello`) by hand,
// `async fn hello()` lets the compiler generate an equivalent one. Each
// `.await` on a Future that returns Pending is a suspension point, like
// `yield` in the Python version:
//
//     def hello():
//         print('Hello, ', end='')
//         yield
//         print('World!')
//         yield
//
// We still poll by hand with a no-op Waker, so no Executor is needed.

use std::future::Future;
use std::pin::{Pin, pin};
use std::task::{Context, Poll, Waker};

// A Future that returns Pending once, then Ready.
// Awaiting it suspends the async fn once, like Python's `yield`.
struct YieldOnce {
    yielded: bool,
}

impl Future for YieldOnce {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        if self.yielded {
            Poll::Ready(())
        } else {
            self.yielded = true;
            Poll::Pending
        }
    }
}

fn yield_now() -> YieldOnce {
    YieldOnce { yielded: false }
}

// Replaces `Hello` and `StateHello`.
async fn hello() {
    print!("Hello, ");
    yield_now().await; // suspend here (StateHello::WORLD)
    println!("World!");
    yield_now().await; // suspend here (StateHello::END)
}

fn main() {
    // A Waker that does nothing. We poll by hand, so nobody needs to be woken.
    let mut cx = Context::from_waker(Waker::noop());

    // Futures from async fn are !Unpin, so Pin::new cannot be used.
    // pin! pins it on the stack instead.
    let mut hello = pin!(hello());

    // Suspend and resume repeatedly.
    let _ = hello.as_mut().poll(&mut cx);
    let _ = hello.as_mut().poll(&mut cx);
    let _ = hello.as_mut().poll(&mut cx);
}

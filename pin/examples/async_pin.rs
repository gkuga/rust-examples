// Why Future::poll takes `Pin<&mut Self>`.
//
// An async block that keeps a reference to its own local variable across an
// `.await` becomes a self-referential struct: the generated state machine stores
// both `x` and `r` (a pointer to `x`). If the future moved between polls, `r`
// would dangle, exactly like `src/main.rs`. So such futures are !Unpin and must
// be pinned before they can be polled.

use std::future::Future;
use std::pin::{pin, Pin};
use std::task::{Context, Poll, Waker};

// A Future that returns Pending once, then Ready.
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

fn main() {
    let fut = async {
        let x = 42;
        let r = &x; // reference into the future's own state
        println!("  before await: &x = {:p}, r = {:p}", &x, r);
        YieldOnce { yielded: false }.await; // suspended here, `x` and `r` are kept
        println!("  after await:  &x = {:p}, r = {:p}", &x, r);
        *r
    };

    // `fut.poll(...)` does not compile: poll needs Pin<&mut Self>.
    // `Pin::new(&mut fut)` does not compile either: the async block is !Unpin.
    // Pin it first (on the stack with pin!, or on the heap with Box::pin).
    let mut fut = pin!(fut);

    // A Waker that does nothing; we just poll by hand like an Executor would.
    let mut cx = Context::from_waker(Waker::noop());

    let mut n = 1;
    loop {
        println!("poll #{n}");
        match fut.as_mut().poll(&mut cx) {
            Poll::Pending => println!("  -> Pending"),
            Poll::Ready(v) => {
                println!("  -> Ready({v})");
                break;
            }
        }
        n += 1;
    }
    // &x is the same address in both polls: pinning guaranteed `r` stays valid.
}

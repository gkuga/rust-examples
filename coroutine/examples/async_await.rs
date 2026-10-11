// The same computation as `callback.rs`, written with async/await.
//
// `x.await + y.await + z.await` reads like ordinary synchronous code.
// Each `.await` is roughly desugared into:
//
//     match h.poll(cx) {
//         Poll::Pending => return Poll::Pending,
//         Poll::Ready(x) => x,
//     }
//
// so a Pending from a nested poll propagates all the way up to the Executor.

use futures::executor::block_on;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

// A Future that returns Pending once before its value becomes ready,
// to show that `.await` suspends and resumes transparently.
struct Value {
    name: &'static str,
    n: u32,
    polled: bool,
}

impl Value {
    fn new(name: &'static str, n: u32) -> Self {
        Value {
            name,
            n,
            polled: false,
        }
    }
}

impl Future for Value {
    type Output = u32;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32> {
        if self.polled {
            println!("{} is ready: {}", self.name, self.n);
            Poll::Ready(self.n)
        } else {
            println!("{} is pending", self.name);
            self.polled = true;
            cx.waker().wake_by_ref(); // ask the Executor to poll again
            Poll::Pending
        }
    }
}

async fn sum() -> u32 {
    let x = Value::new("x", 1);
    let y = Value::new("y", 2);
    let z = Value::new("z", 3);

    x.await + y.await + z.await
}

fn main() {
    // block_on is a minimal Executor from the futures crate:
    // it polls the Future until it returns Ready.
    let sum = block_on(sum());
    println!("sum = {sum}");
}

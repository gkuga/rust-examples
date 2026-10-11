use futures::future::{BoxFuture, FutureExt};
use futures::task::{ArcWake, waker_ref};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

// (1) Holds the function's state and variables.
// "Hello, World!" has no variables, so only the progress state is kept.
struct Hello {
    state: StateHello,
}

// (2) Execution state of the function.
// HELLO is the initial state; WORLD and END correspond to the
// first and second `yield` in the Python version.
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

    // (3) The actual "function call".
    // Runs the code for the current state and transitions to the next one.
    // Returns Poll::Pending while it can be resumed, Poll::Ready when finished.
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        match (*self).state {
            StateHello::HELLO => {
                print!("Hello, ");
                // transition to WORLD
                (*self).state = StateHello::WORLD;
                Poll::Pending // can be called again
            }
            StateHello::WORLD => {
                println!("World!");
                // transition to END
                (*self).state = StateHello::END;
                Poll::Pending // can be called again
            }
            StateHello::END => {
                Poll::Ready(()) // finished
            }
        }
    }
}

// (1) Unit of execution (task) in async/await.
struct Task {
    hello: Mutex<BoxFuture<'static, ()>>,
}

impl Task {
    fn new() -> Self {
        let hello = Hello::new();
        Task {
            hello: Mutex::new(hello.boxed()),
        }
    }
}

// Does nothing. ArcWake is the trait used for scheduling tasks.
impl ArcWake for Task {
    fn wake_by_ref(_arc_self: &Arc<Self>) {}
}

fn main() {
    // initialization
    let task = Arc::new(Task::new());
    let waker = waker_ref(&task);
    // (2) poll requires a Context, which can be built from an ArcWake reference.
    let mut ctx = Context::from_waker(&waker);
    let mut hello = task.hello.lock().unwrap();

    // (3) Suspend and resume repeatedly.
    // Calling poll three times prints "Hello, World!", same as the Python version.
    hello.as_mut().poll(&mut ctx);
    hello.as_mut().poll(&mut ctx);
    hello.as_mut().poll(&mut ctx);
}

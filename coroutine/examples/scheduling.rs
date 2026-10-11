use futures::future::{BoxFuture, FutureExt};
use futures::task::{ArcWake, waker_ref};
use std::future::Future;
use std::pin::Pin;
use std::sync::mpsc::{Receiver, SyncSender, sync_channel}; // (1)
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

// (1) Function and types for the communication channel.
// Receiver and SyncSender are the receiving and sending endpoints.
// mpsc stands for Multiple Producers, Single Consumer.

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

// Example 5-2: Task type
// For simplicity, the Task itself also acts as the Waker.
struct Task {
    // coroutine to run
    future: Mutex<BoxFuture<'static, ()>>, // (1)
    // channel for scheduling onto the Executor
    sender: SyncSender<Arc<Task>>, // (2)
}

// (1) The coroutine (Future) to run. The Executor keeps running it until it completes.
// (2) Channel used to hand the Task to the Executor for scheduling.

impl ArcWake for Task {
    // (3) Schedule itself by sending its own Arc to the Executor.
    fn wake_by_ref(arc_self: &Arc<Self>) {
        // schedule itself
        let self0 = arc_self.clone();
        arc_self.sender.send(self0).unwrap();
    }
}

// Example 5-3: Executor type
// (1) Only holds the endpoints of the channel (run queue) that carries Tasks.
struct Executor {
    // run queue
    sender: SyncSender<Arc<Task>>,
    receiver: Receiver<Arc<Task>>,
}

impl Executor {
    fn new() -> Self {
        // create the channel; the queue holds at most 1024 entries
        let (sender, receiver) = sync_channel(1024);
        Executor {
            sender: sender.clone(),
            receiver,
        }
    }

    // (2) Create a Spawner for creating new Tasks.
    // Returns an object that creates a Task and enqueues it,
    // similar to the thread-spawning `spawn` function in Rust.
    fn get_spawner(&self) -> Spawner {
        Spawner {
            sender: self.sender.clone(),
        }
    }

    // (3) Receive Tasks from the channel and run them.
    // Since Task and Waker are the same here, build a Waker from the Task,
    // a Context from the Waker, and call poll with that Context.
    fn run(&self) {
        // receive Tasks from the channel and run them in order
        while let Ok(task) = self.receiver.recv() {
            // create the context
            let mut future = task.future.lock().unwrap();
            let waker = waker_ref(&task);
            let mut ctx = Context::from_waker(&waker);
            // call poll to run it
            let _ = future.as_mut().poll(&mut ctx);
        }
    }
}

// Example 5-4: Spawner type
// (1) Simply holds the sending endpoint of the run queue.
struct Spawner {
    sender: SyncSender<Arc<Task>>,
}

impl Spawner {
    // (2) Box the Future, wrap it in a Task, and enqueue it.
    fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        let future = future.boxed(); // box the Future
        let task = Arc::new(Task {
            // create the Task
            future: Mutex::new(future),
            sender: self.sender.clone(),
        });

        // enqueue onto the run queue
        self.sender.send(task).unwrap();
    }
}

// Same as the previous Hello, except it now schedules itself.
impl Future for Hello {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        match (*self).state {
            StateHello::HELLO => {
                print!("Hello, ");
                (*self).state = StateHello::WORLD;
                cx.waker().wake_by_ref(); // enqueue itself onto the run queue
                return Poll::Pending;
            }
            StateHello::WORLD => {
                println!("World!");
                (*self).state = StateHello::END;
                cx.waker().wake_by_ref(); // enqueue itself onto the run queue
                return Poll::Pending;
            }
            StateHello::END => {
                return Poll::Ready(());
            }
        }
    }
}

fn main() {
    let executor = Executor::new();
    executor.get_spawner().spawn(Hello::new());
    executor.run();
}

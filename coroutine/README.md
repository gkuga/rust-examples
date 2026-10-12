# Rust Coroutine Example

Rust has no coroutines in the language itself, but a function that behaves like one
can be implemented as a **function with state**. This example rewrites the Python
coroutine version of "Hello, World!" using the `Future` trait.

```python
def hello():
    print('Hello, ', end='')
    yield
    print('World!')
    yield

h = hello()
h.__next__()
h.__next__()
```

## How it works

- `Hello` holds the function's state (`StateHello`).
- `StateHello` represents how far the function has run:
  `HELLO` (initial) → `WORLD` (after 1st `yield`) → `END` (after 2nd `yield`).
- `poll` runs the code for the current state and transitions to the next state.
  - Returns `Poll::Pending` while the function can be resumed.
  - Returns `Poll::Ready(value)` once everything has finished.
- `Task` is the unit of execution in async/await, and `ArcWake` is the trait used for
  scheduling. Here `wake_by_ref` does nothing; it exists only so a `Context` can be
  built (via `waker_ref` + `Context::from_waker`), which `poll` requires.

Calling `poll` three times prints `Hello, World!`, the same as the Python version.

## Scheduling version (`examples/scheduling.rs`)

Instead of calling `poll` by hand, the coroutine is driven by a simple scheduler
built from three parts:

- **Executor**: receives runnable `Task`s from a channel (the run queue) and `poll`s
  the `Future` inside each one.
- **Task**: the unit of scheduling. It holds the `Future` and a `SyncSender` to the
  run queue. For simplicity, `Task` also acts as the **Waker**: `wake_by_ref` sends
  its own `Arc` back to the run queue.
- **Spawner**: boxes a `Future`, wraps it in a `Task`, and enqueues it (like `spawn`).

`Hello::poll` now calls `cx.waker().wake_by_ref()` after each state transition, so it
re-enqueues itself and the Executor runs it again automatically until it returns
`Poll::Ready`.

Note: the Executor keeps its own `SyncSender`, so the channel never closes and `run()`
blocks on `recv()` forever after `Hello, World!` is printed. Stop it with Ctrl-C.

## Callbacks vs async/await (`examples/callback.rs`, `examples/async_await.rs`)

With callbacks, each dependent step is nested inside the previous one
("callback hell"):

```rust
x.poll(|a| {
    y.poll(|b| {
        z.poll(|c| {
            a + b + c
        })
    })
})
```

With async/await, the same computation reads like synchronous code:

```rust
x.await + y.await + z.await
```

Each `.await` roughly expands to the following, so a `Pending` from a nested `poll`
propagates all the way up to the Executor:

```rust
match h.poll(cx) {
    Poll::Pending => return Poll::Pending,
    Poll::Ready(x) => x,
}
```

In `async_await.rs`, each `Value` returns `Pending` once before becoming ready, so the
output shows the suspend/resume happening behind the plain-looking `+` expression.

## Executor, Waker, and `.await` in detail

### Where is the Executor?

In `async_await.rs`, the Executor is `futures::executor::block_on` in `main`. It runs on
the calling thread (no background runtime) and drives a single Future, roughly like this:

```rust
fn block_on<F: Future>(fut: F) -> F::Output {
    let mut fut = pin!(fut);
    let waker = /* a Waker that unparks the current thread */;
    let mut cx = Context::from_waker(&waker);
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(v) => return v,
            Poll::Pending => thread::park(), // sleep until woken (returns at once if already woken)
        }
    }
}
```

Rust itself only provides the `Future` trait and the `async`/`await` syntax; there is **no
built-in Executor**. It comes from a library (`futures`, `tokio`, ...) or your own code.

| | Book's `Executor` (`scheduling.rs`) | `block_on` (`async_await.rs`) |
|---|---|---|
| Provided by | our own code | the `futures` crate |
| Runs on | the thread that calls `run()` | the thread that calls `block_on` |
| Tasks | many (a channel is the run queue) | exactly one |
| What `wake` does | sends its own `Arc<Task>` to the channel | unparks the thread |
| Finishes | never (`recv` blocks forever) | when the Future returns `Ready` |

### What `cx.waker().wake_by_ref()` does

It tells the Executor "poll me again". `cx.waker()` returns the `&Waker` the Executor put
in the `Context`; `wake_by_ref` calls it without consuming it (`wake` consumes it).

- **It does not call `poll`.** It is an ordinary synchronous call that only *schedules*
  the task (enqueues it, or marks the thread as woken) and returns at once. `poll` then
  returns `Pending`, control goes back to the Executor, and the Executor polls again
  later. So `poll` never re-enters itself.
- **Without it, the program hangs.** The Executor does not re-poll a `Pending` Future on
  its own. Removing the line from `async_await.rs` prints only `x is pending` and then
  `block_on` parks forever.
- Waking right before returning `Pending` (as these examples do) just yields once. Real
  I/O futures instead store a clone of the Waker and call it when the OS reports the I/O
  is ready, so the Executor can run other tasks in the meantime.
- Multi-threaded Executors may pick up a woken task on another thread before the current
  `poll` returns. They still never poll one Future concurrently (the book's `Task` wraps
  the Future in a `Mutex`; tokio tracks a "notified while running" state).

### `.await` is not a busy loop

`x.await` roughly expands to:

```rust
let a = loop {
    match x.poll(cx) {
        Poll::Ready(v) => break v,
        Poll::Pending => yield, // pseudo-code: the whole async fn returns Pending
    }
};
```

On `Pending`, the enclosing Future saves where it stopped and returns `Pending` to the
Executor. Each turn of this loop is separated by a return to the Executor, which polls
again only after a wake:

```
block_on              sum()                      x
────────              ─────                      ─
poll sum ──────────→  reach x.await
                      poll x ─────────────────→  "x is pending", wake_by_ref()
                      ←──────────── Pending
←───── Pending        (state saved)
(woken) poll sum ──→  resume at x.await
                      poll x ─────────────────→  "x is ready: 1"
                      ←──────────── Ready(1)
                      a = 1, go on to y.await
```

So "`.await` waits until the value is ready" is how it looks from inside `sum()`. The
thread is never spinning on `poll`; while one Future waits, the Executor is free to run
others.

## Run

```sh
cargo run                       # manual poll version
cargo run --example scheduling  # scheduling version (Ctrl-C to exit)
cargo run --example callback    # callback-based version
cargo run --example async_await # async/await version
```

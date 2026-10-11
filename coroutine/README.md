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

## Run

```sh
cargo run                       # manual poll version
cargo run --example scheduling  # scheduling version (Ctrl-C to exit)
cargo run --example callback    # callback-based version
cargo run --example async_await # async/await version
```

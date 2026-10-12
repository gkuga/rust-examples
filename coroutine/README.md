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

## Without Arc (`examples/noop_waker.rs`)

In `src/main.rs`, `Arc`, `Task`, and `Mutex` are not needed for the program itself:

- `poll` needs a `&mut Context`, and a `Context` is built from a `Waker`.
- The book builds the Waker with `futures::task::waker_ref`, which requires an
  `&Arc<impl ArcWake>`. So `Task` and `Arc` exist only to obtain a Waker, not because
  anything is shared.
- The Waker is never used: `wake_by_ref` does nothing and `main` calls `poll` by hand.

`noop_waker.rs` is the same program using `std::task::Waker::noop()`, with no `Arc`,
`Task`, `Mutex`, or `futures` crate:

```rust
let mut cx = Context::from_waker(Waker::noop());
let mut hello = Hello::new();
let _ = Pin::new(&mut hello).poll(&mut cx); // Hello is Unpin, so Pin::new is fine
```

Why the book uses `Arc` anyway (likely reasons):

- **No no-op Waker back then.** The book targets Rust 1.50; `Waker::noop()` was
  stabilized in Rust 1.85. Implementing `ArcWake` was an easy way to get a Waker.
- **Preparing for the next section.** In `scheduling.rs`, `Arc<Task>` is really needed:
  the run queue, the Executor, and the Waker all hold the same Task.

## With async/await (`examples/await_hello.rs`)

`noop_waker.rs` with the hand-written state machine replaced by `async fn`. The compiler
generates an equivalent state machine, and the code reads like the Python version:

```rust
async fn hello() {
    print!("Hello, ");
    yield_now().await; // suspend (StateHello::WORLD)
    println!("World!");
    yield_now().await; // suspend (StateHello::END)
}
```

| | `noop_waker.rs` | `await_hello.rs` |
|---|---|---|
| State machine | `Hello` + `StateHello`, written by hand | generated from `async fn` |
| Suspension point | returning `Poll::Pending` | `yield_now().await` |
| Pinning | `Pin::new` (`Hello` is `Unpin`) | `pin!` (async futures are `!Unpin`) |
| Hand-written `Future` | `Hello` | only `YieldOnce`, the thing to await on |

`.await` needs a Future that actually returns `Pending` to suspend, so a minimal one
(`YieldOnce`: `Pending` once, then `Ready`) is still written by hand.

## With an Executor (`examples/await_block_on.rs`)

`await_hello.rs` with the three manual `poll` calls replaced by `block_on(hello())`.

`.await` cannot be used in `main` (a plain `fn`): at the top level, something has to
receive `Pending` and call `poll` again. That is the Executor, here
`futures::executor::block_on`.

The one required change is in `YieldOnce`, which now wakes before returning `Pending`:

```rust
self.yielded = true;
cx.waker().wake_by_ref(); // without this, block_on sleeps forever after the first poll
Poll::Pending
```

A Future that returns `Pending` must arrange to be woken. `await_hello.rs` gets away
without it only because it polls by hand with a no-op Waker. Since every suspension
point goes through `YieldOnce`, this single line fixes all the `.await`s in `hello()`.
(Passing `noop_waker.rs`'s `Hello` to `block_on` would instead need a wake in each
`Pending` arm, which is the book's self-scheduling `Hello`. See `hello_block_on.rs`.)

| | `await_hello.rs` | `await_block_on.rs` |
|---|---|---|
| Who calls `poll` | `main`, three times by hand | `block_on` (Executor) |
| Waker | `Waker::noop()` | `block_on`'s Waker (unparks the thread) |
| Pinning | `pin!` in `main` | done inside `block_on` |
| `YieldOnce` wakes before `Pending` | no (not needed) | yes (required) |
| `futures` crate | not needed | needed |

## Hand-written Future with an Executor (`examples/hello_block_on.rs`)

`noop_waker.rs`'s `Hello`, with its hand-written `poll` kept, driven by `block_on`
instead of calling `poll` by hand:

```rust
block_on(Hello::new());
```

`Hello` is itself a Future, so it can be passed to `block_on` directly. `await_hello.rs`
replaces the state machine with async; this one keeps the state machine and lets an
Executor drive it.

A hand-written Future can also be `.await`ed from async code, e.g.
`block_on(async { Hello::new().await; })`. That behaves the same here (it only adds one
more Future layer), so the async block is needed only when there is more to do, such as
awaiting several Futures in sequence.

The one required change is a `cx.waker().wake_by_ref()` in each `Pending` arm of
`Hello::poll` (the book's self-scheduling `Hello`). Without it, `block_on` sleeps forever
after the first poll.

| | `noop_waker.rs` | `hello_block_on.rs` |
|---|---|---|
| `Hello::poll` | wakes nothing | wakes before each `Pending` |
| Who calls `poll` | `main`, three times by hand | `block_on` (Executor) |
| Waker | `Waker::noop()` | `block_on`'s Waker (unparks the thread) |
| `futures` crate | not needed | needed |

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

## Why poll-based? (zero-cost futures)

### Overhead and zero-cost

**Overhead** is any cost beyond the work itself, compared with the best hand-written code
for the same job. For async code the usual suspects are:

- **Heap allocation** (`Box::new`, `Arc::new`)
- **Dynamic dispatch** (`dyn Trait` calls go through a vtable and cannot be inlined)
- **Synchronization** (`Mutex` locks, atomic reference counts in `Arc`)

**Zero-cost abstraction** (in Stroustrup's words): "What you don't use, you don't pay for.
And further: What you do use, you couldn't hand code any better." For futures, it means
code built by composing Futures is as efficient as a hand-written state machine like
`Hello` + `StateHello`.

### Example: wait for two operations (join)

**Callbacks (push): the child calls the parent.** `f` and `g` may finish in either order,
so both callbacks must share a slot for the results:

```rust
let shared = Arc::new(Mutex::new((None, None))); // allocation + lock
let s1 = shared.clone();
f.on_complete(move |a| { s1.lock().unwrap().0 = Some(a); /* if both done, continue */ });
let s2 = shared.clone();
g.on_complete(move |b| { s2.lock().unwrap().1 = Some(b); /* if both done, continue */ });
```

Each callback is also registered with the event loop, which must hold callbacks of many
different types, so they become `Box<dyn FnOnce>` (allocation + dynamic dispatch). This
happens at every point of composition.

**Polling (pull): the parent calls the child.**

```rust
enum Join<F: Future, G: Future> {
    BothRunning(F, G),        // owns f and g by value
    FirstDone(F::Output, G),  // stores f's result in its own field
    SecondDone(F, G::Output),
    Done,
}
```

- `Join` owns `f` and `g`, so it calls `f.poll(cx)` directly: static dispatch, inlinable.
- Results come back as return values and are stored in `Join` itself. Nothing is shared,
  so no `Arc` or `Mutex`.
- Inner Futures are embedded by value, so the whole thing is one enum whose size is known
  at compile time. No allocation.
- The only notification channel is one Waker per **task**, so the only allocation is
  when a task is created (the `BoxFuture` in the book's `Task`), however many Futures are
  composed inside it.

```
Callbacks (child -> parent)          Polling (parent -> child)

  f --+                                Join
      +--> shared state --> next        +-- f (owned)
  g --+    (Arc<Mutex<..>>)             +-- g (owned)
```

The key difference is the direction of calls. With callbacks, children need a reference
back to the parent's continuation, and several children point to the same parent:
shared ownership. With polling, ownership stays a tree and results flow up as return
values, which fits Rust's ownership model and costs nothing extra. In GC languages the
garbage collector absorbs the cost of sharing, so callbacks are less of a problem there.

Note: `examples/callback.rs` is actually zero-cost, because it calls each callback
immediately. The overhead appears as soon as callbacks have to be *stored* to be called
later, as real async I/O requires.

### How the design evolved

| | futures 0.1 (2016) | `std::future` (Rust 1.36+) |
|---|---|---|
| `poll` | `poll(&mut self)` | `poll(self: Pin<&mut Self>, cx: &mut Context)` |
| Waking | `task::park()` (implicit, thread-local), `unpark()` | `Waker` passed explicitly in `Context` |
| Composition | combinators (`and_then`, `join`, ...) | async/await (Rust 1.39+) |

`Pin` was added for async/await: holding a reference across `.await` makes the generated
state machine self-referential (see `../pin`). Combinator chains could not borrow across
steps, so they did not need it.

### Other languages

| Language | Model | Drives the coroutine | Executor / runtime | Notes |
|---|---|---|---|---|
| **Rust** | async/await, stackless | poll (pull) + Waker | none built in (`tokio`, `futures`, own) | state machine embedded by value; Futures are lazy |
| JavaScript | async/await on Promise | push: resolving a Promise schedules continuations | built-in event loop | GC; async functions start running immediately |
| C# | async/await on `Task` | push: continuations registered on `Task` | built-in thread pool | compiler-generated state machine; `ValueTask` reduces allocations |
| Python (asyncio) | async/await, stackless | hybrid: a `Task` drives the coroutine with `send()`, and Futures wake the Task via `add_done_callback` | `asyncio` event loop (library) | the Task / callback split is close to Rust's Task / Waker |
| Kotlin | `suspend` functions, stackless | push: compiled to continuation-passing style (`Continuation.resumeWith`) | library (`kotlinx.coroutines`) | no `await` keyword at call sites |
| C++20 | `co_await`, stackless | customizable via `promise_type` / awaiters | none in the standard library | frames are usually heap-allocated (may be elided) |
| Go | goroutines, stackful | runtime scheduler (preemptive) | built-in M:N scheduler | no async/await; blocking calls look synchronous |
| Java 21+ | virtual threads, stackful | runtime scheduler | built-in (JVM) | `CompletableFuture` is the older push-based API |

Rust is unusual in combining stackless coroutines, a poll-based model, and no built-in
runtime. That combination is what makes futures zero-cost and usable without an
allocator-heavy runtime, at the price of `Pin` and choosing an Executor yourself.

### Go vs Rust + tokio

The languages above fall into two families:

| | async/await family (Rust, JS, C#, Python, Kotlin, C++20) | Lightweight-thread family (Go, Java 21 virtual threads, Erlang) |
|---|---|---|
| Coroutine | stackless | stackful (each goroutine has its own stack) |
| Where it can suspend | only at `.await` | at any blocking call (Go 1.14+ can also preempt) |
| Code style | `async` / `.await` written explicitly | looks like ordinary synchronous code |
| Function "color" | yes: async vs plain functions | no |
| Where suspended state lives | compiler-generated state machine | the goroutine's own stack |

"Function color" is the split described in Bob Nystrom's "What Color is Your Function?"
(2015): `.await` is only allowed inside async functions (that is why `main` cannot
`.await`), so async spreads to callers.

**Go has the same machinery, hidden in the runtime.** When a goroutine blocks in
`conn.Read()`, the runtime parks it and runs another goroutine on the OS thread. The
netpoller (epoll on Linux) detects readiness and puts the goroutine back on a run queue.
That is the Executor, run queue, Waker, and Reactor from this crate, built into the
language.

**Rust once had this too.** Pre-1.0 Rust had green threads (`libgreen`), removed in 2014
(RFC 230), because they cost:

- a mandatory runtime (Rust also targets bare-metal and C-callable libraries)
- a stack per thread (more memory than a state machine that stores only what it needs)
- stack switching when calling C (like Go's cgo overhead)

**tokio provides a Go-like runtime as a library.**

| Feature | Go runtime | tokio |
|---|---|---|
| Spawn a lightweight unit | `go f()` | `tokio::spawn(async { ... })` |
| Scheduler | M:N, work stealing | M:N, work stealing (multi-thread runtime) |
| I/O readiness | netpoller | Reactor (via mio) |
| Timers (`sleep`, `timeout`) | managed by the runtime | managed by the runtime |

tokio's scheduler drew on Go's design (see the tokio blog post "Making the Tokio scheduler
10x faster", 2019). What still differs is the unit being scheduled:

| | Go | tokio |
|---|---|---|
| Unit | goroutine, stackful | Future (state machine), stackless |
| Memory per unit | small stack that grows on demand | exactly the state machine's size, one allocation |
| Switching | anywhere; the runtime can preempt | only at `.await` (cooperative) |
| Function color | none | async vs plain |
| Relation to the language | built in, cannot be removed | one library among others (e.g. Embassy for embedded) |

**Timers vs preemption.** Both runtimes manage timers: `time.Sleep` and
`tokio::time::sleep` park only that goroutine/task and the runtime wakes it when the time
comes. Timers have no default duration; you pass one each time. The real difference is
preemption, i.e. stopping a task that keeps running:

| | Switches based on | Value (implementation detail, may change) |
|---|---|---|
| tokio | **count**: each `.await` spends a cooperative budget; when it runs out, the task yields | budget of 128 |
| Go | **time**: a monitor thread (sysmon) preempts a goroutine that runs too long | about 10 ms |

So with tokio, a computation that never `.await`s is never switched out, and blocking
calls (`std::fs::read`, `std::thread::sleep`, heavy loops) stall the whole worker thread
and every task on it. The division of labor is:

| tokio takes care of | The programmer takes care of |
|---|---|
| when and on which thread each task is polled | not blocking inside async code (use `spawn_blocking` or async APIs such as `tokio::fs`) |
| calling Wakers when I/O or timers are ready | yielding during long computations (`.await` points, or move the work to another thread) |
| load balancing across worker threads | deciding which functions are async |

In short: Go favors ease of writing and lets the runtime absorb the complexity and cost;
Rust favors zero cost and leaves some of the complexity (function color, `Pin`, choosing
an Executor, cooperating with it) to the programmer.

### References

- Aaron Turon, [Zero-cost futures in Rust](https://aturon.github.io/blog/2016/08/11/futures/) (2016)
- Aaron Turon, [Designing futures for Rust](https://aturon.github.io/blog/2016/09/07/futures-design/) (2016):
  why callbacks were rejected, tasks as "a single, permanent callback", cancellation by drop
- Tatsuya Kawano, [Rustのゼロコスト抽象化の効果をアセンブラで確認](https://blog.rust-jp.rs/tatsuya6502/posts/2019-12-zero-cost-abstraction/) (2019):
  static dispatch, closures, and `Option` niche optimization checked in assembly
- Melvin E. Conway, "Design of a Separable Transition-Diagram Compiler", CACM (1963):
  early use of the term "coroutine"
- Ana Lúcia de Moura and Roberto Ierusalimschy, "Revisiting Coroutines", ACM TOPLAS (2009):
  classification (symmetric/asymmetric, stackful/stackless). Rust's async/await is
  asymmetric and stackless.
- Related design patterns: Active Object (scheduler + activation queue + futures) and
  Reactor (I/O readiness -> wake the task)

## Run

```sh
cargo run                          # manual poll version
cargo run --example noop_waker     # manual poll version without Arc
cargo run --example await_hello    # manual poll version with async/await
cargo run --example await_block_on # async/await driven by block_on
cargo run --example hello_block_on # hand-written Future driven by block_on
cargo run --example scheduling     # scheduling version (Ctrl-C to exit)
cargo run --example callback       # callback-based version
cargo run --example async_await    # async/await version
```

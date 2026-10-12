# async/await Examples

Based on the tokio tutorial: https://tokio.rs/tokio/tutorial/spawning

## Blocks and block modifiers

### Plain blocks

A plain `{ ... }` is a **block expression**: it runs immediately and evaluates to its
last expression.

```rust
let x = {
    let a = 1;
    let b = 2;
    a + b // no `;`, so this is the block's value
};        // x = 3
```

Uses:

- Compute a value without leaking temporary variables.
- Limit a scope: variables are dropped at the end of the block (e.g. release a lock early).
  ```rust
  {
      let mut guard = mutex.lock().unwrap();
      *guard += 1;
  } // guard is dropped here, so the lock is released
  ```
- Function bodies and `if` / `match` arms are blocks too, which is why the last
  expression becomes the return value.

### Modifiers

A keyword before a block adds a property to it:

| Modifier | What changes | Value of the block |
|---|---|---|
| `unsafe { ... }` | unsafe operations are allowed inside | the inner value |
| `const { ... }` | evaluated at compile time (Rust 1.79+) | the inner value |
| `'label: { ... }` | can exit early with `break 'label value` (Rust 1.65+) | the inner value |
| `async { ... }` | **execution is deferred** | **a Future** |

`async` is the odd one out: `unsafe { 1 + 2 }` is `3`, but `async { 1 + 2 }` is a Future
that will produce `3` when polled. It is more accurate to think of it as "wrapping the
block in a Future" than as a modifier.

| | `{ 1 + 2 }` | `async { 1 + 2 }` |
|---|---|---|
| Runs | immediately | when polled |
| Value | `3` (`i32`) | `impl Future<Output = i32>` |
| `.await` inside | not allowed (unless the enclosing fn is async) | allowed |

`move` in `async move { ... }` means the same as in `move || ...`: captured variables are
moved into the block.

`loop { ... }` looks similar but is not a block modifier: it is a loop expression whose
body is a block (`break value` gives the loop its value).

## async block vs closure vs async fn

### An async block is a Future, not a closure

`async { ... }` is an expression that evaluates to a **Future** (a value of an anonymous
type implementing `Future`). It is not a function and cannot be called.

```rust
let fut = async {
    println!("running"); // not printed yet: the body runs only when polled
};
println!("created");
block_on(fut);           // prints "running" now; `fut` is consumed
```

- The body does not run until the Future is polled (by `.await` or an Executor).
- The `Output` type is the type of the last expression (`async { 1 + 2 }` is
  `impl Future<Output = i32>`; a trailing `;` makes it `()`).
- `.await` can only be written inside an `async` block or `async fn`.

| | Closure `\|x\| { ... }` | Async block `async { ... }` |
|---|---|---|
| Trait | `Fn` / `FnMut` / `FnOnce` | `Future` |
| How it runs | called: `f(x)` | polled (via `.await` or an Executor) |
| Arguments | yes | no |
| Runs | every time it is called, from the start | once; done after it returns `Ready` |
| Can suspend midway | no | yes, at each `.await` |

What they have in common:

- Both have an anonymous type, so they are passed as `impl Fn(..)` / `impl Future`.
- Both capture variables from the environment; `move` transfers ownership
  (`move || ...` / `async move { ... }`).
- Neither runs at the point where it is written.

### A closure that returns a Future is a function

Combining the two gives a closure that returns an async block. Calling it creates a new
Future each time:

```rust
let fut  = async { println!("hi") };    // a Future (a value)
let make = || async { println!("hi") }; // a function that returns a Future

block_on(fut);    // runs `fut`; it cannot be used again
block_on(make()); // make() creates a Future, which is then run
block_on(make()); // call it again for another, new Future
```

Since Rust 1.85, `async || { ... }` (an async closure) can be written directly.

### async fn is on the function side too

`async fn` is roughly "a function that returns an async block":

```rust
async fn hello() {
    Hello::new().await;
}

// roughly the same as
fn hello() -> impl Future<Output = ()> {
    async {
        Hello::new().await;
    }
}
```

| | Function (makes a Future) | Future (the value made) |
|---|---|---|
| async fn | `hello` | `hello()` |
| Closure | `make` | `make()` |
| async block | — | `async { ... }` itself |

So `async fn hello` and `|| async { ... }` are **functions**, and calling them gives a
**Future**. An async block is written directly as the Future.

### In this crate

`src/main.rs` passes an async block to `tokio::spawn`:

```rust
tokio::spawn(async move {
    process(socket, db).await;
});
```

`async move { ... }` creates a Future that owns `socket` and `db`, and `tokio::spawn`
hands it to tokio's Executor as a new task. Nothing inside runs until the Executor polls it.

See also `../coroutine` for hand-written Futures, Wakers, and a minimal Executor.

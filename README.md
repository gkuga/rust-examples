# rust-examples

Small Rust programs for understanding language features one at a time. Each directory
is an independent Cargo crate:

```sh
cd <directory>
cargo run                     # src/main.rs
cargo run --example <name>    # examples/<name>.rs, if any
```

## Ownership and memory

| Directory | Topic |
|---|---|
| [borrowing](borrowing/) | Mutable/immutable borrows, reborrowing, and NLL (non-lexical lifetimes) |
| [box](box/) | `Box<T>` basics (stack vs heap, moving a Box), and `Box<dyn Fn>` for closures |
| [rc](rc/) | `Rc<T>`: shared ownership and reference counts |
| [manually_drop](manually_drop/) | `ManuallyDrop<T>`: suppressing the automatic drop |
| [maybeuninit](maybeuninit/) | `MaybeUninit<T>`: initializing a value through a raw pointer |
| [pin](pin/) | `Pin`, `Unpin`, `PhantomPinned`, and why self-referential types need pinning |

## Closures and macros

| Directory | Topic |
|---|---|
| [closures](closures/) | Closure syntax and type inference, compared with functions |
| [macros](macros/) | A minimal `macro_rules!` macro |

## Concurrency

| Directory | Topic |
|---|---|
| [thread](thread/) | `std::thread::spawn` and `join` |
| [mutex](mutex/) | Sharing a counter between threads with `Arc<Mutex<T>>` |
| [atomic](atomic/) | Memory ordering: `Relaxed` vs `Release`/`Acquire`, with a reorder litmus test |
| [deadlock](deadlock/) | How deadlocks happen with `RwLock` guards, and how to avoid them |

## Async

| Directory | Topic |
|---|---|
| [coroutine](coroutine/) | Coroutines as state machines with `Future`, Executor/Task/Waker, async/await, and why Rust futures are poll-based |
| [async-await](async-await/) | async blocks vs closures vs `async fn`, block modifiers, and the tokio tutorial |

Suggested reading order: `thread` → `coroutine` → `pin` → `async-await`.

## Systems

| Directory | Topic |
|---|---|
| [mkfifo](mkfifo/) | Named pipes (FIFO) between a parent and a forked child process (`nix`) |
| [wasm-runtime](wasm-runtime/) | A tiny WebAssembly runtime: parse a module and execute a function |

## Notes and explanations

Longer write-ups live in each directory's README:

- **Coroutines and async** ([coroutine/README.md](coroutine/README.md))
  - [Executor, Waker, and `.await` in detail](coroutine/README.md#executor-waker-and-await-in-detail)
  - [Why poll-based? (zero-cost futures)](coroutine/README.md#why-poll-based-zero-cost-futures):
    overhead, callbacks vs polling, how the design evolved, other languages
  - [Go vs Rust + tokio](coroutine/README.md#go-vs-rust--tokio): stackful vs stackless,
    function color, timers vs preemption
- **Pinning** ([pin/README.md](pin/README.md))
  - [The core idea](pin/README.md#the-core-idea)
  - [Unpin](pin/README.md#unpin) (auto trait, which types are not `Unpin`)
  - [When to add PhantomPinned](pin/README.md#when-to-add-phantompinned)
- **async syntax** ([async-await/README.md](async-await/README.md))
  - [Blocks and block modifiers](async-await/README.md#blocks-and-block-modifiers)
  - [async block vs closure vs async fn](async-await/README.md#async-block-vs-closure-vs-async-fn)
- **Locks** ([deadlock/README.md](deadlock/README.md)): RAII and guards

Note: `async-await/src/main.rs` currently does not compile (`process` is called with two
arguments but takes one); it is an in-progress step of the tokio tutorial.

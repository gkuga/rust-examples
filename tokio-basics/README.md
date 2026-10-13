# Tokio Basics

Small programs for using Tokio, following the book's section on async libraries and the
[Tokio tutorial](https://tokio.rs/tokio/tutorial). Each one is an example:

```sh
cargo run --example <name>
```

| Example | What it shows | Runs for |
|---|---|---|
| `echo_server` | Echo server on `127.0.0.1:10000`; compare with `../async-epoll` (same server on raw epoll) | until Ctrl-C |
| `sleep_bad` | `std::thread::sleep` inside async code blocks the worker thread | 10 s |
| `sleep_good` | `tokio::time::sleep` parks only the task | 10 s |
| `std_mutex` | `std::sync::Mutex` is fine when the lock is not held across `.await` | instant |
| `tokio_mutex` | `tokio::sync::Mutex` is needed to hold a lock across `.await` | 10 s |
| `oneshot` | `oneshot` channel as "a value determined in the future" | 10 s |
| `spawn_blocking` | Run blocking functions on dedicated threads so async tasks keep running | ~20 s |
| `redis_server` | mini-redis server from the [Tokio tutorial](https://tokio.rs/tokio/tutorial/shared-state): a task per connection, shared state in `Arc<Mutex<HashMap>>` | until Ctrl-C |
| `hello_redis` | mini-redis client from the [Tokio tutorial](https://tokio.rs/tokio/tutorial/hello-tokio): `set` then `get` | instant (needs `redis_server`) |

Try the echo server from another terminal (bash):

```sh
bash -c 'exec 3<>/dev/tcp/127.0.0.1/10000; echo hello >&3; head -n1 <&3'
```

Try the mini-redis pair:

```sh
cargo run --example redis_server   # terminal 1
cargo run --example hello_redis    # terminal 2: got value from the server; result=Some(b"world")
```

`redis_server` uses `std::sync::Mutex` for the shared store, which is fine because the
lock is released before any `.await` (see "std Mutex vs tokio Mutex" below).

## Key points

### Thread pool

Tokio's default runtime starts one worker thread per CPU core. Each worker runs an
Executor that takes Tasks from the run queue (a thread pool), instead of creating a
thread per connection. This is why async is cheaper than thread-per-connection code,
even though the echo server could be written with plain threads.

### Never block in async code

A blocking call (`std::thread::sleep`, blocking I/O, long computations) occupies the
worker thread, so other tasks on it cannot run. That slows everything down and can even
deadlock.

| Blocking (avoid in async) | Async alternative |
|---|---|
| `std::thread::sleep` | `tokio::time::sleep(..).await` |
| blocking I/O / CPU-heavy work that must block | `tokio::task::spawn_blocking(..)` |
| `std::sync::mpsc` channels | `tokio::sync::{mpsc, oneshot, broadcast, watch}` |

`sleep_bad` and `sleep_good` look the same when run alone (both take 10 s); the
difference shows when other tasks share the worker thread.

### std Mutex vs tokio Mutex

- Lock, update, unlock with no `.await` in between: `std::sync::Mutex` is fine and faster
  (`std_mutex`).
- Holding the lock across `.await`: use `tokio::sync::Mutex` (`tokio_mutex`). With std's
  Mutex, a task parked while holding the lock would leave the other tasks blocked in
  `lock()` on the worker threads: a deadlock.
- Rust catches this at compile time. Switching `tokio_mutex.rs` to `std::sync::Mutex`
  fails with:
  ```
  error: future cannot be sent between threads safely
  note: future is not `Send` as this value is used across an await
        ... has type `std::sync::MutexGuard<'_, u64>` which is not `Send`
  ```
  `tokio::spawn` may move a task between worker threads, so its Future must be `Send`,
  and a `MutexGuard` kept across `.await` becomes part of that Future.

### Tokio channels

| Channel | Producers / consumers | Notes |
|---|---|---|
| `mpsc` | many / one | async version of `std::sync::mpsc::channel` |
| `oneshot` | one / one | sends exactly one value; works like a Future for a value set later |
| `broadcast` | many / many | every receiver gets every value |
| `watch` | one / many | receivers see only the latest value |

With `oneshot`, if one end is dropped, using the other end returns an error.

### spawn_blocking output

`do_print` keeps printing `wake up` every second while 32 `do_block` calls sleep for 10
seconds each on blocking threads:

```
wake up        (x9)
finished: 0
finished: 1
...
```

## Notes

- `Cargo.toml` uses `tokio = { version = "1", features = ["full"] }` (the book pins
  1.4.0). `full` enables everything, which increases compile time and binary size.
- Library APIs change faster than concepts; check the official Tokio docs for current
  details.

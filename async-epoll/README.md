# IO Multiplexing with async/await (epoll)

A concurrent echo server built from scratch on `epoll`, following the book's section
"IO multiplexing and async/await". It reuses `Task`, `Executor`, and `Spawner` from
`../coroutine/examples/scheduling.rs` and adds `IOSelector`, which plays the role of the
Reactor.

**Linux only** (`epoll`, `eventfd`). macOS has `kqueue` instead and cannot build this, so
the `Makefile` runs everything in a Linux container (Docker required).

```
Executor <── run queue <────────── wake ─────────┐
   │                                             │
  poll                                     IOSelector (own thread)
   v                                         epoll_wait
Task/Waker ──> IO queue (+ eventfd) ──────> fd -> Waker map
 (Futures)     register / unregister
```

## Components

| Type | Role |
|---|---|
| `IOSelector` | Runs `epoll_wait` in a dedicated thread. Keeps a map from fd to `Waker`, and calls `wake_by_ref` when an fd becomes readable, which puts the Task back on the run queue |
| `IOOps` | Requests to `IOSelector`: `ADD(flags, fd, waker)` / `REMOVE(fd)`, pushed to the IO queue |
| `eventfd` | Wakes `IOSelector` out of `epoll_wait` when a request is queued. A channel cannot be used because the thread is blocked in `epoll_wait`, not in `recv` |
| `AsyncListener` / `Accept` | Non-blocking `accept`. On `WouldBlock`, registers the listening socket with `IOSelector` and returns `Pending` |
| `AsyncReader` / `ReadLine` | Non-blocking `read_line`. On `WouldBlock`, registers the socket and returns `Pending` |
| `Task` / `Executor` / `Spawner` | Same as the scheduling section |

`EPOLLONESHOT` disables an fd after one event, so each `register` re-arms it with
`EpollCtlMod` when the fd is already in epoll (`EEXIST`).

`main` is plain async/await: accept in a loop and `spawn` one Task per connection that
echoes each line back. The epoll details are hidden inside the Futures.

## Run

```sh
make test                  # start the server, run two concurrent clients, check idle CPU, stop
```

Or by hand, in two terminals:

```sh
make run                   # terminal 1: start the server (Ctrl-C to stop)
make client MSG="hi there" # terminal 2: send a line and print the echo
make stop                  # stop the server
```

The server listens on `127.0.0.1:10000` inside the container, which is not reachable from
the host, so `make client` connects from inside the container (bash's `/dev/tcp`). On a
Linux machine, plain `cargo run` also works.

`make test` output:

```
hello from B
hello from A
--- idle CPU of the server (should be about 0%):
 0.0 /tmp/target/debug/async-epoll
--- server log:
accept: 127.0.0.1:34934
accept: 127.0.0.1:34936
read: 127.0.0.1:34936, hello from A
read: 127.0.0.1:34934, hello from B
close: 127.0.0.1:34936
close: 127.0.0.1:34934
```

Two clients connected at the same time are served concurrently by the single Executor
thread.

## Notes

- **The `nix` version is pinned to 0.20.** The book's code uses that API
  (`epoll_create1` / `epoll_ctl` / `epoll_wait` functions, `nix::Error::Sys(Errno)`).
  Newer `nix` versions replaced them with an `Epoll` type and `Errno` as the error type.
- **Busy-loop fix (differs from the book).** The book's `IOSelector::select` never reads
  the eventfd, so its counter stays above 0. epoll is level-triggered by default, so
  `epoll_wait` keeps returning immediately and the selector thread used ~100% CPU even
  when idle (measured 99.7%). The book's text says the notification is cleared by writing
  0, but writing to an eventfd adds to the counter; it has to be read. `select` now calls
  `read_eventfd` in the eventfd branch (measured 0.0% idle).
- **Explicit `'_` lifetimes (differs from the book).** The book writes
  `fn accept(&self) -> Accept` and `fn read_line(&mut self) -> ReadLine`. Newer Rust warns
  that this hides the lifetime borrowed from `self`, so they return `Accept<'_>` and
  `ReadLine<'_>`.
- Only reading is asynchronous; writing uses a blocking `BufWriter`, as in the book.

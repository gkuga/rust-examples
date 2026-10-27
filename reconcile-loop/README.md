# Reconcile Loop

A controller publishes a **desired state**. A reconciler makes its **actual state** match
it and reports back. Whenever the desired state changes, the reconciler does this again.
Kubernetes controllers work this way.

```sh
cargo run    # about 5.5 s
```

```
controller ──desired (watch)──▶ next() ──▶ reconcile ──▶ report (mpsc)
    ▲                                                        │
    └──────────────────────── printed ◀──────────────────────┘
```

Here the desired state is a set of named workers, each with a tick interval. The loop
keeps one task per worker.

## The loop

```rust
while let Some(a) = control.next().await {
    // 1. check the desired state: a bad entry is left out, not fatal
    // 2. publish it on a watch channel, so running workers see it
    // 3. drop the tasks of departed workers (AbortOnDrop aborts them)
    // 4. spawn tasks for new workers; running ones are left alone
    // 5. report which revision is applied, with any problems
}
```

## Key points

| Point | Where | What the run shows |
|---|---|---|
| **The revision is a hash of the content.** `next()` returns an assignment only when its revision differs from the last one it returned. | `Assignment::new`, `Control::next` | At 1000 ms the same content is sent again, and no reconcile follows. |
| **A `watch` channel keeps only the latest value.** The loop sees the newest desired state, not every step on the way to it. | `Control::next` | At 4000 ms two updates are sent back to back, and only the second is reconciled. `worker_d` never starts. |
| **Diff, don't rebuild.** Only departed workers stop and only new ones start. | steps 3–4 | At 1500 ms, `worker_a` keeps ticking while `worker_b` starts. |
| **Running tasks read the latest state** instead of a copy taken at spawn. | `worker` | At 3000 ms `worker_a` speeds up without a restart. Its next tick still waits out the old 400 ms sleep, because a change applies from the following tick. |
| **One bad entry does not block the rest.** It is left out and reported. | step 1 | `worker_c` has interval 0: `applied … with problems`, while `worker_a` and `worker_b` keep running. |
| **Dropping the handle stops the task.** | `AbortOnDrop` | Removing a worker from the map stops it, and when the loop ends, every worker stops with it. |
| **The loop ends when the controller is gone.** | `rx.changed()` returns `Err` | At 5500 ms the sender is dropped, and `next()` returns `None`. |

`main` uses the `current_thread` runtime so the output is deterministic. On that runtime,
the two back-to-back updates both happen before the loop task runs again.

## Two channels, two meanings

- **Desired state: `watch`.** The loop only needs the newest state. Intermediate states
  are useless, so overwriting them is a feature.
- **Reports: `mpsc`.** Every report is a separate event, and none should be lost.

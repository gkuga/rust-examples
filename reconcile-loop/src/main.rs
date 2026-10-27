// A reconcile loop: receive the desired state, make the actual state match it,
// report the result, and repeat whenever the desired state changes.
//
//   controller ──desired (watch)──▶ next() ──▶ reconcile ──▶ report (mpsc)
//       ▲                                                        │
//       └──────────────────────── printed ◀──────────────────────┘
//
// The desired state here is a set of named workers, each with a tick interval.
// The loop keeps one task per worker: it spawns new ones, drops departed ones,
// and lets running ones pick up a changed interval without being restarted.

use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use tokio::time::{Instant, sleep};

/// A worker's name, unique within an assignment.
type WorkerId = String;
/// A worker's tick interval, in milliseconds.
type IntervalMs = u64;

/// The desired state, as the controller publishes it.
#[derive(Clone, Debug)]
struct Assignment {
    /// A hash of the content: the same content always has the same revision.
    revision: String,
    /// Worker name -> tick interval in milliseconds.
    workers: BTreeMap<WorkerId, IntervalMs>,
}

impl Assignment {
    fn new(workers: &[(&str, IntervalMs)]) -> Assignment {
        let workers: BTreeMap<WorkerId, IntervalMs> =
            workers.iter().map(|(n, ms)| (n.to_string(), *ms)).collect();
        let mut h = DefaultHasher::new();
        workers.hash(&mut h);
        Assignment {
            revision: format!("{:08x}", h.finish() as u32),
            workers,
        }
    }
}

/// What the reconciler reports back for each revision it handled.
#[derive(Debug)]
struct Report {
    revision: String,
    problems: Vec<String>,
}

/// The receiving side of the control plane: hands out each new revision once.
struct Control {
    rx: watch::Receiver<Option<Assignment>>,
    yielded: Option<String>,
}

impl Control {
    /// The next assignment whose revision differs from the last one handed
    /// out. `None` once the controller is gone.
    async fn next(&mut self) -> Option<Assignment> {
        loop {
            // Mark the current value as seen, then look at it.
            let current = self.rx.borrow_and_update().clone();
            if let Some(a) = current
                && self.yielded.as_deref() != Some(a.revision.as_str())
            {
                self.yielded = Some(a.revision.clone());
                return Some(a);
            }
            // Wait for a new value; an error means the sender was dropped.
            self.rx.changed().await.ok()?;
        }
    }
}

/// Aborts the task when dropped, so removing it from the map stops it.
struct AbortOnDrop(JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// One worker: tick at the interval the *latest* assignment gives it.
async fn worker(name: WorkerId, current: watch::Receiver<Arc<Assignment>>, start: Instant) {
    loop {
        // Read the latest assignment at every tick instead of capturing the
        // interval once, so a change applies without restarting the task.
        let interval = current.borrow().workers.get(&name).copied();
        let Some(ms) = interval else { return };
        sleep(Duration::from_millis(ms)).await;
        println!(
            "{:>6} ms  {name}: tick (every {ms} ms)",
            start.elapsed().as_millis()
        );
    }
}

/// The reconciler: the reconcile loop itself.
async fn reconciler(mut control: Control, report: mpsc::UnboundedSender<Report>, start: Instant) {
    // The latest assignment, shared with every worker task.
    let (current, _) = watch::channel(Arc::new(Assignment::new(&[])));
    // The actual state: one running task per worker.
    let mut tasks: BTreeMap<WorkerId, AbortOnDrop> = BTreeMap::new();

    while let Some(a) = control.next().await {
        let t = start.elapsed().as_millis();
        println!(
            "{t:>6} ms  reconcile: revision {} {:?}",
            a.revision, a.workers
        );

        // 1. Check the desired state. A bad entry does not stop the others:
        //    it is left out and reported.
        let mut problems = Vec::new();
        let mut desired = a.clone();
        desired.workers.retain(|name, ms| {
            if *ms == 0 {
                problems.push(format!("{name}: interval must be > 0"));
                false
            } else {
                true
            }
        });

        // 2. Publish it, so running workers see the new intervals.
        current.send_replace(Arc::new(desired.clone()));

        // 3. Departed workers: dropping the handle aborts the task.
        tasks.retain(|name, _| {
            let keep = desired.workers.contains_key(name);
            if !keep {
                println!("{t:>6} ms  reconcile: stop {name}");
            }
            keep
        });

        // 4. New workers: spawn. Running ones are left alone.
        for name in desired.workers.keys() {
            if tasks.contains_key(name) {
                continue;
            }
            println!("{t:>6} ms  reconcile: start {name}");
            let task = tokio::spawn(worker(name.clone(), current.subscribe(), start));
            tasks.insert(name.clone(), AbortOnDrop(task));
        }

        // 5. Report which revision is applied, with any problems.
        let _ = report.send(Report {
            revision: a.revision.clone(),
            problems,
        });
    }
    println!(
        "{:>6} ms  reconcile: controller gone, stopping",
        start.elapsed().as_millis()
    );
    // `tasks` is dropped here, which aborts every worker.
}

/// The controller: change the desired state over time, then go away.
async fn controller(assignment_tx: watch::Sender<Option<Assignment>>, start: Instant) {
    let at = |ms: u64| sleep(Duration::from_millis(ms).saturating_sub(start.elapsed()));

    println!("--- 0 ms: one worker");
    assignment_tx.send_replace(Some(Assignment::new(&[("worker_a", 400)])));

    at(1000).await;
    println!("--- 1000 ms: the same content again (same revision: no reconcile)");
    assignment_tx.send_replace(Some(Assignment::new(&[("worker_a", 400)])));

    at(1500).await;
    println!("--- 1500 ms: add worker_b, and worker_c with a bad interval");
    assignment_tx.send_replace(Some(Assignment::new(&[
        ("worker_a", 400),
        ("worker_b", 700),
        ("worker_c", 0),
    ])));

    at(3000).await;
    println!("--- 3000 ms: worker_a gets faster, worker_c removed");
    assignment_tx.send_replace(Some(Assignment::new(&[
        ("worker_a", 150),
        ("worker_b", 700),
    ])));

    at(4000).await;
    println!("--- 4000 ms: two updates back to back (watch keeps only the latest)");
    assignment_tx.send_replace(Some(Assignment::new(&[
        ("worker_a", 150),
        ("worker_b", 700),
        ("worker_d", 100),
    ])));
    assignment_tx.send_replace(Some(Assignment::new(&[("worker_b", 700)])));

    at(5500).await;
    println!("--- 5500 ms: controller gone");
    // `assignment_tx` is dropped here, which ends the reconcile loop.
}

// current_thread keeps the output deterministic: the controller's two
// back-to-back updates at 4000 ms both happen before the loop runs again.
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let start = Instant::now();
    let (assignment_tx, assignment_rx) = watch::channel::<Option<Assignment>>(None);
    let (report_tx, mut report_rx) = mpsc::unbounded_channel();

    let reconciler_task = tokio::spawn(reconciler(
        Control {
            rx: assignment_rx,
            yielded: None,
        },
        report_tx,
        start,
    ));

    // The controller's side of the report: print what the reconciler applied.
    let printer_task = tokio::spawn(async move {
        while let Some(r) = report_rx.recv().await {
            let t = start.elapsed().as_millis();
            if r.problems.is_empty() {
                println!("{t:>6} ms  report: applied {}", r.revision);
            } else {
                println!(
                    "{t:>6} ms  report: applied {} with problems {:?}",
                    r.revision, r.problems
                );
            }
        }
    });

    controller(assignment_tx, start).await;
    reconciler_task.await.unwrap();
    printer_task.await.unwrap();
}

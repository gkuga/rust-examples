// Example 4-4: banker.rs
// Banker's Algorithm for resource allocation (single-threaded core)

pub const NRES: usize = 3;
pub const NTH: usize = 3;

pub struct Resource<const NRES: usize, const NTH: usize> {
    available: [usize; NRES],
    allocation: [[usize; NRES]; NTH], // Does not guarantee consistency if modified separately
    max: [[usize; NRES]; NTH],
}

impl<const NRES: usize, const NTH: usize> Resource<NRES, NTH> {
    pub fn new(available: [usize; NRES], max: [[usize; NRES]; NTH]) -> Self {
        Resource {
            available,
            allocation: [[0; NRES]; NTH],
            max,
        }
    }

    // ❶ Check if the current state is safe
    fn is_safe(&self) -> bool {
        let mut finish = [false; NTH]; // ❷ Thread i can finish & release if finish[i] is true
        let mut work = self.available; // ❸ Simulate available resources

        loop {
            // ❹ Look for a not-yet-finished thread whose remaining need fits in work
            let mut found = false;
            let mut num_true = 0;
            for (i, alloc) in self.allocation.iter().enumerate() {
                if finish[i] {
                    num_true += 1;
                    continue;
                }

                // ❺ need[j] = max[i][j] - allocation[i][j]
                // ❻ thread i can be satisfied if work[j] >= need[j] for all resources j
                let need = self.max[i].iter().zip(alloc).map(|(m, a)| m - a);
                let is_avail = work.iter().zip(need).all(|(w, n)| *w >= n);
                if is_avail {
                    // ❼ Thread i finishes and returns all its resources to work
                    found = true;
                    finish[i] = true;
                    for (w, a) in work.iter_mut().zip(alloc) {
                        *w += *a;
                    }
                    break;
                }
            }

            if num_true == NTH {
                // ❽ Every thread can finish -> safe state
                return true;
            }

            if !found {
                // ❾ No thread can make progress -> unsafe (deadlock/starvation possible)
                break;
            }
        }

        false
    }

    // ❿ Thread `id` acquires one unit of `resource`
    pub fn take(&mut self, id: usize, resource: usize) -> bool {
        // Validate thread id / resource id
        if id >= NTH
            || resource >= NRES
            || self.available[resource] == 0
            || self.max[id][resource] == self.allocation[id][resource]
        {
            return false;
        }

        // ⓫ Tentatively acquire the resource
        self.allocation[id][resource] += 1;
        self.available[resource] -= 1;

        if self.is_safe() {
            true // Acquisition succeeded
        } else {
            // Acquisition would be unsafe; roll back
            self.allocation[id][resource] -= 1;
            self.available[resource] += 1;
            false
        }
    }

    // ⓬ Thread `id` releases one unit of `resource`
    pub fn release(&mut self, id: usize, resource: usize) {
        // Validate thread id / resource id
        if id >= NTH || resource >= NRES || self.allocation[id][resource] == 0 {
            return;
        }

        self.allocation[id][resource] -= 1;
        self.available[resource] += 1;
    }
}

// Single-threaded demonstration of the Banker's Algorithm.
pub fn single_thread_example() {
    // 3 resource types, 3 threads.
    // available: the banker starts with [3, 3, 3] units on hand.
    // max:       the maximum each thread may ever request.
    let available = [3, 3, 3];
    let max = [
        [3, 1, 1], // thread 0
        [1, 3, 1], // thread 1
        [1, 1, 3], // thread 2
    ];
    let mut res: Resource<NRES, NTH> = Resource::new(available, max);

    // Each thread acquires what it needs, does some "work", then releases.
    // take() only succeeds when the resulting state stays safe.
    let ok0 = res.take(0, 0); // thread 0 takes resource 0
    let ok1 = res.take(1, 1); // thread 1 takes resource 1
    let ok2 = res.take(2, 2); // thread 2 takes resource 2
    println!("take results: t0={ok0}, t1={ok1}, t2={ok2}");

    // Release them again.
    res.release(0, 0);
    res.release(1, 1);
    res.release(2, 2);
    println!("released all resources");
}

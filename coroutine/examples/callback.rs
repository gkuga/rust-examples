// Callback-based asynchronous style ("callback hell").
//
// Assume `poll` takes a callback and calls it with the result once the value
// is determined. Each dependent step must be nested inside the previous
// callback, so the code drifts to the right as the number of steps grows.

struct Value {
    name: &'static str,
    n: u32,
}

impl Value {
    fn new(name: &'static str, n: u32) -> Self {
        Value { name, n }
    }

    // Calls `callback` with the result once the value is ready.
    // (A real implementation would call it later, e.g. when I/O completes.)
    fn poll<R>(&self, callback: impl FnOnce(u32) -> R) -> R {
        println!("{} is ready: {}", self.name, self.n);
        callback(self.n)
    }
}

fn main() {
    let x = Value::new("x", 1);
    let y = Value::new("y", 2);
    let z = Value::new("z", 3);

    let sum = x.poll(|a| {
        y.poll(|b| {
            z.poll(|c| {
                a + b + c
            })
        })
    });

    println!("sum = {sum}");
}

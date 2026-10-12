// Storing closures in a Box<dyn Fn>.
//
// Every closure has its own unnamed type, even if two closures have the same
// signature. `Box<dyn Fn(u64) -> u64>` erases that type, so closures can be:
//   - stored in a variable or field whose type must be written out
//   - put together in one Vec

// A trait object type for "a function from u64 to u64".
type Op = Box<dyn Fn(u64) -> u64>;

fn main() {
    // `move` copies `extra` into the closure, so the closure owns its data
    // and does not borrow from main's stack frame.
    let extra = 10;
    let calculator: Op = Box::new(move |y| 5 * y + extra);
    println!("calculator(3) = {}", calculator(3));

    // `|n| n + 1` and `|n| n * n` are different types, so a Vec of them is only
    // possible after boxing them as the same trait object type.
    let operations: Vec<(&str, Op)> = vec![
        ("n + 1", Box::new(|n| n + 1)),
        ("n * n", Box::new(|n| n * n)),
        ("calculator", calculator),
    ];

    println!("Execute functions in the list:");
    for (name, op) in &operations {
        println!("  {name}: {}", op(10));
    }
}

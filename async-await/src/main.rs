// async/await syntax: blocks, async blocks, closures, and async fn.
// Each section matches a heading in README.md.

use futures::executor::block_on;
use std::future::Future;

// --- async fn is on the function side ---

async fn hello() -> u32 {
    1
}

// roughly the same as `async fn hello`
fn hello_desugared() -> impl Future<Output = u32> {
    async { 1 }
}

fn main() {
    // --- Plain blocks: run immediately and evaluate to the last expression ---
    let x = {
        let a = 1;
        let b = 2;
        a + b // no `;`, so this is the block's value
    };
    println!("plain block: x = {x}");

    // --- Modifiers: `async` turns the block into a Future ---
    let a = { 1 + 2 }; // i32, computed now
    let b = async { 1 + 2 }; // impl Future<Output = i32>, not computed yet
    println!("{{ 1 + 2 }} = {a}, async {{ 1 + 2 }} = {} (after block_on)", block_on(b));

    // --- An async block is a Future, not a closure: it runs only when polled ---
    let fut = async {
        println!("  running");
    };
    println!("  created");
    block_on(fut); // prints "running" now; `fut` is consumed
    // block_on(fut); // error: use of moved value

    // --- A closure that returns a Future is a function ---
    let make = || async { println!("  hi from a new Future") };
    block_on(make()); // make() creates a Future, which is then run
    block_on(make()); // call it again for another, new Future

    // async closure (Rust 1.85+)
    let make2 = async || println!("  hi from an async closure");
    block_on(make2());

    // --- async fn: `hello` is a function, `hello()` is a Future ---
    println!("hello() = {}, hello_desugared() = {}", block_on(hello()), block_on(hello_desugared()));
}

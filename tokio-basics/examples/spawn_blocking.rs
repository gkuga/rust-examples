// Running blocking functions with spawn_blocking.
//
// Some work has to call blocking functions. spawn_blocking runs them on
// threads dedicated to blocking work, so the async worker threads stay free
// and do_print keeps printing every second while do_block sleeps.

// a blocking function
fn do_block(n: u64) -> u64 {
    let ten_secs = std::time::Duration::from_secs(10);
    std::thread::sleep(ten_secs);
    n
}

// an async function
async fn do_print() {
    let sec = std::time::Duration::from_secs(1);
    for _ in 0..20 {
        tokio::time::sleep(sec).await;
        println!("wake up");
    }
}

#[tokio::main]
pub async fn main() {
    // call the blocking function
    let mut v = Vec::new();
    for n in 0..32 {
        let t = tokio::task::spawn_blocking(move || do_block(n)); // on a blocking-only thread
        v.push(t);
    }

    // call the async function, which prints periodically
    let p = tokio::spawn(do_print());

    for t in v {
        let n = t.await.unwrap();
        println!("finished: {}", n);
    }

    p.await.unwrap()
}

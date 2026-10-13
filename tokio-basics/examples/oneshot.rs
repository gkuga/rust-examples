// tokio::sync::oneshot: a value that is determined in the future.
//
// mpsc, broadcast, and watch are channels for sending and synchronizing.
// oneshot is closer to a Future itself: a single value set at some point
// later. If either end is dropped, using the other end returns an error.

use tokio::sync::oneshot; // import oneshot

// sets the value at some point in the future (here, after a sleep)
async fn set_val_later(tx: oneshot::Sender<i32>) {
    let ten_secs = std::time::Duration::from_secs(10);
    tokio::time::sleep(ten_secs).await;
    if let Err(_) = tx.send(100) {
        // write the value with send
        println!("failed to send");
    }
}

#[tokio::main]
pub async fn main() {
    let (tx, rx) = oneshot::channel(); // separate send and receive ends, as with other channels

    tokio::spawn(set_val_later(tx)); // pass the sender to the function that sets the value

    match rx.await {
        // wait until the value is set
        Ok(n) => {
            println!("n = {}", n);
        }
        Err(e) => {
            println!("failed to receive: {}", e);
            return;
        }
    }
}

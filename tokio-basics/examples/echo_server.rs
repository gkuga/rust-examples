// Echo server with Tokio.
//
// Apart from `.await` on async calls, it looks almost the same as code using
// the standard library. Compare with ../async-epoll, which implements the
// same server by hand on epoll.

use tokio::io::{AsyncBufReadExt, AsyncWriteExt}; // traits for async buffered read/write
use tokio::io;
use tokio::net::TcpListener; // async TCP listener

#[tokio::main] // required for an async main
async fn main() -> io::Result<()> {
    // listen on port 10000; written almost like std's TcpListener
    let listener = TcpListener::bind("127.0.0.1:10000").await.unwrap();

    loop {
        // accept a TCP connection
        let (mut socket, addr) = listener.accept().await?;
        println!("accept: {}", addr);

        // spawn an async task
        tokio::spawn(async move {
            // buffered reader/writer, as with the standard library
            let (r, w) = socket.split(); // split into read and write halves
            let mut reader = io::BufReader::new(r);
            let mut writer = io::BufWriter::new(w);

            let mut line = String::new();
            loop {
                line.clear(); // Tokio's read_line appends to the string, so clear it
                match reader.read_line(&mut line).await {
                    // read one line asynchronously
                    Ok(0) => {
                        // connection closed
                        println!("closed: {}", addr);
                        return;
                    }
                    Ok(_) => {
                        print!("read: {}, {}", addr, line);
                        writer.write_all(line.as_bytes()).await.unwrap();
                        writer.flush().await.unwrap();
                    }
                    Err(e) => {
                        // error
                        println!("error: {}, {}", addr, e);
                        return;
                    }
                }
            }
        });
    }
}

extern crate nix;
extern crate tempdir;

use std::fs::OpenOptions;
use std::io::prelude::*;
use std::process::exit;
use std::thread::sleep;
use std::time::Duration;

use nix::sys::stat;
use nix::sys::wait::waitpid;
use nix::unistd;
use nix::unistd::{fork, getpid, getppid, ForkResult};
use tempdir::TempDir;

fn main() {
    println!("Main({}) stared", getpid());

    let tmp_dir = TempDir::new("test_fifo").unwrap();
    let fifo_path = tmp_dir.path().join("foo.pipe");

    match unistd::mkfifo(&fifo_path, stat::Mode::S_IRWXU) {
        Ok(_) => println!("created {:?}", fifo_path),
        Err(err) => println!("Error creating fifo: {}", err),
    }

    let child_pid = match fork() {
        Ok(ForkResult::Parent { child, .. }) => {
            println!("Main({}) forked a child({})", getpid(), child);
            child
        }
        Ok(ForkResult::Child) => {
            println!("Child({}) started. PPID is {}.", getpid(), getppid());
            sleep(Duration::from_secs(3));
            let mut file = OpenOptions::new().read(true).open(&fifo_path).unwrap();
            let mut contents = String::new();
            file.read_to_string(&mut contents).unwrap();
            println!("{}", contents);
            exit(0);
        }
        Err(_) => panic!("fork() failed"),
    };

    {
        let mut file = OpenOptions::new().write(true).open(&fifo_path).unwrap();
        file.write_all(b"Hello, world!").unwrap();
    }

    match waitpid(child_pid, None) {
        Ok(status) => println!("Child exited ({:?}).", status),
        Err(_) => println!("waitpid() failed"),
    }
}

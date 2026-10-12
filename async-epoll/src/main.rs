// IO multiplexing with async/await: a concurrent echo server built on epoll.
//
// Task, Executor, and Spawner are reused from the scheduling section
// (see ../coroutine/examples/scheduling.rs). This adds IOSelector, which
// watches file descriptors with epoll in a dedicated thread and calls the
// registered Waker when an event occurs, putting the Task back on the run queue.
//
// Linux only (epoll, eventfd).

use futures::{
    future::{BoxFuture, FutureExt},
    task::{waker_ref, ArcWake},
};
use nix::{
    errno::Errno,
    sys::{
        epoll::{
            epoll_create1, epoll_ctl, epoll_wait, EpollCreateFlags, EpollEvent, EpollFlags,
            EpollOp,
        },
        eventfd::{eventfd, EfdFlags}, // for eventfd
    },
    unistd::{read, write},
};
use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    io::{BufRead, BufReader, BufWriter, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    os::unix::io::{AsRawFd, RawFd},
    pin::Pin,
    sync::{
        mpsc::{sync_channel, Receiver, SyncSender},
        Arc, Mutex,
    },
    task::{Context, Poll, Waker},
};

// eventfd is a Linux-specific event notification interface. The kernel
// holds an 8-byte counter, and a read event fires while it is greater than 0.
// It is read and written with the read and write system calls. Here it is used
// to notify IOSelector.

// Write to an eventfd. This only calls the write system call, but it is more
// verbose than in C.
fn write_eventfd(fd: RawFd, n: usize) {
    // convert usize to *const u8
    let ptr = &n as *const usize as *const u8;
    let val = unsafe { std::slice::from_raw_parts(ptr, std::mem::size_of_val(&n)) };
    // write system call
    write(fd, &val).unwrap();
}

// Read an eventfd: reading its 8-byte counter resets it to 0.
// Not in the book's code; see the comment in IOSelector::select.
fn read_eventfd(fd: RawFd) {
    let mut buf = [0u8; 8];
    read(fd, &mut buf).unwrap();
}

// ---------------------------------------------------------------------------
// 5.3.2.1 IOSelector
// ---------------------------------------------------------------------------

// Operations requested to IOSelector: register or remove a file descriptor
// (and its Waker). They are pushed to the IO queue.
enum IOOps {
    ADD(EpollFlags, RawFd, Waker), // add to epoll
    REMOVE(RawFd),                 // remove from epoll
}

// Holds the map from file descriptor to Waker, so the right Waker can be
// called when an event occurs on a file descriptor.
struct IOSelector {
    wakers: Mutex<HashMap<RawFd, Waker>>, // fd to waker
    // The IO queue in Figure 5-3. VecDeque instead of LinkedList: it allocates
    // and frees memory far less often when used as a queue.
    queue: Mutex<VecDeque<IOOps>>, // IO queue
    epfd: RawFd,                   // fd of epoll
    event: RawFd,                  // fd of eventfd
}

impl IOSelector {
    fn new() -> Arc<Self> {
        let s = IOSelector {
            wakers: Mutex::new(HashMap::new()),
            queue: Mutex::new(VecDeque::new()),
            epfd: epoll_create1(EpollCreateFlags::empty()).unwrap(),
            // create eventfd
            // eventfd(initial value, flags) just calls the system call of the same name
            event: eventfd(0, EfdFlags::empty()).unwrap(),
        };
        let result = Arc::new(s);
        let s = result.clone();

        // spawn a thread for epoll: event management runs in a separate thread
        std::thread::spawn(move || s.select());

        result
    }

    // watch with epoll: add the fd to epoll and associate it with a Waker
    fn add_event(
        &self,
        flag: EpollFlags, // epoll flags
        fd: RawFd,        // fd to watch
        waker: Waker,
        wakers: &mut HashMap<RawFd, Waker>,
    ) {
        // shortcuts
        let epoll_add = EpollOp::EpollCtlAdd;
        let epoll_mod = EpollOp::EpollCtlMod;
        let epoll_one = EpollFlags::EPOLLONESHOT;

        // With EPOLLONESHOT, once an event fires, no more events are reported
        // for that fd until it is re-armed. The fd stays registered in epoll.
        let mut ev = EpollEvent::new(flag | epoll_one, fd as u64);

        // add to the watch list
        if let Err(err) = epoll_ctl(self.epfd, epoll_add, fd, &mut ev) {
            match err {
                nix::Error::Sys(Errno::EEXIST) => {
                    // already added: re-arm it. This is needed to re-enable an
                    // event disabled by EPOLLONESHOT. Remembering which fds were
                    // added would save system calls; kept simple to show EPOLLONESHOT.
                    epoll_ctl(self.epfd, epoll_mod, fd, &mut ev).unwrap();
                }
                _ => {
                    panic!("epoll_ctl: {}", err);
                }
            }
        }

        assert!(!wakers.contains_key(&fd));
        wakers.insert(fd, waker); // associate fd with waker
    }

    // remove from epoll: stop watching the fd and drop its Waker
    fn rm_event(&self, fd: RawFd, wakers: &mut HashMap<RawFd, Waker>) {
        let epoll_del = EpollOp::EpollCtlDel;
        let mut ev = EpollEvent::new(EpollFlags::empty(), fd as u64);
        epoll_ctl(self.epfd, epoll_del, fd, &mut ev).ok();
        wakers.remove(&fd);
    }

    // select: watches file descriptors in a dedicated thread
    fn select(&self) {
        // shortcuts
        let epoll_in = EpollFlags::EPOLLIN;
        let epoll_add = EpollOp::EpollCtlAdd;

        // add the eventfd to epoll first
        let mut ev = EpollEvent::new(epoll_in, self.event as u64);
        epoll_ctl(self.epfd, epoll_add, self.event, &mut ev).unwrap();

        let mut events = vec![EpollEvent::empty(); 1024];
        // wait for events
        while let Ok(nfds) = epoll_wait(self.epfd, &mut events, -1) {
            let mut t = self.wakers.lock().unwrap();
            for n in 0..nfds {
                if events[n].data() == self.event as u64 {
                    // eventfd: process add/remove requests

                    // Fix (not in the book's code): read the eventfd to reset its
                    // counter to 0. epoll is level-triggered by default, so while the
                    // counter is above 0 the eventfd stays readable and epoll_wait
                    // returns immediately forever, spinning this thread at ~100% CPU
                    // even when idle. (Writing 0, as the book's text says, does not
                    // help: write adds to the counter.)
                    read_eventfd(self.event);

                    let mut q = self.queue.lock().unwrap();
                    while let Some(op) = q.pop_front() {
                        match op {
                            // add
                            IOOps::ADD(flag, fd, waker) => {
                                self.add_event(flag, fd, waker, &mut t)
                            }
                            // remove
                            IOOps::REMOVE(fd) => self.rm_event(fd, &mut t),
                        }
                    }
                } else {
                    // a watched fd: wake the task (push it to the run queue)
                    let data = events[n].data() as i32;
                    let waker = t.remove(&data).unwrap();
                    waker.wake_by_ref();
                }
            }
        }
    }

    // register an fd and Waker; used by Futures to enqueue requests
    fn register(&self, flags: EpollFlags, fd: RawFd, waker: Waker) {
        let mut q = self.queue.lock().unwrap();
        q.push_back(IOOps::ADD(flags, fd, waker));
        write_eventfd(self.event, 1);
    }

    // unregister an fd and its Waker
    fn unregister(&self, fd: RawFd) {
        let mut q = self.queue.lock().unwrap();
        q.push_back(IOOps::REMOVE(fd));
        write_eventfd(self.event, 1);
    }
}

// Requests to IOSelector are enqueued and signaled with eventfd, not a channel,
// because IOSelector also has to watch file descriptors with epoll.

// ---------------------------------------------------------------------------
// 5.3.2.2 Futures
// ---------------------------------------------------------------------------

// Async listener: just a TcpListener and the IOSelector.
struct AsyncListener {
    listener: TcpListener,
    selector: Arc<IOSelector>,
}

impl AsyncListener {
    // wraps TcpListener's initialization
    fn listen(addr: &str, selector: Arc<IOSelector>) -> AsyncListener {
        // listen address
        let listener = TcpListener::bind(addr).unwrap();

        // make it non-blocking, which enables asynchronous programming
        listener.set_nonblocking(true).unwrap();

        AsyncListener {
            listener: listener,
            selector: selector,
        }
    }

    // returns a Future that accepts a connection; it does not accept here.
    // `accept().await` performs the accept asynchronously.
    fn accept(&self) -> Accept {
        Accept { listener: self }
    }
}

impl Drop for AsyncListener {
    // just unregister from epoll
    fn drop(&mut self) {
        self.selector.unregister(self.listener.as_raw_fd());
    }
}

// Async accept: accept without blocking. On success, return the read/write
// streams and the address. If there is no connection to accept, register the
// listening socket with epoll and suspend.
struct Accept<'a> {
    listener: &'a AsyncListener,
}

impl<'a> Future for Accept<'a> {
    // return type
    type Output = (
        AsyncReader,          // async read stream
        BufWriter<TcpStream>, // write stream
        SocketAddr,           // address
    );

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // accept without blocking
        match self.listener.listener.accept() {
            // non-blocking because of set_nonblocking above
            Ok((stream, addr)) => {
                // accepted: return reader/writer objects and the address
                let stream0 = stream.try_clone().unwrap();
                Poll::Ready((
                    AsyncReader::new(stream0, self.listener.selector.clone()),
                    BufWriter::new(stream),
                    addr,
                ))
            }
            Err(err) => {
                // nothing to accept (WouldBlock): register with epoll and suspend
                if err.kind() == std::io::ErrorKind::WouldBlock {
                    self.listener.selector.register(
                        EpollFlags::EPOLLIN,
                        self.listener.listener.as_raw_fd(),
                        cx.waker().clone(),
                    );
                    Poll::Pending
                } else {
                    panic!("accept: {}", err);
                }
            }
        }
    }
}

// Async reader. Only reading is made asynchronous (writing is omitted for
// simplicity), so the read stream is an AsyncReader.
struct AsyncReader {
    fd: RawFd,
    reader: BufReader<TcpStream>,
    selector: Arc<IOSelector>,
}

impl AsyncReader {
    fn new(stream: TcpStream, selector: Arc<IOSelector>) -> AsyncReader {
        // make it non-blocking
        stream.set_nonblocking(true).unwrap();
        AsyncReader {
            fd: stream.as_raw_fd(),
            reader: BufReader::new(stream),
            selector: selector,
        }
    }

    // returns a Future that reads one line
    fn read_line(&mut self) -> ReadLine {
        ReadLine { reader: self }
    }
}

impl Drop for AsyncReader {
    fn drop(&mut self) {
        self.selector.unregister(self.fd);
    }
}

// Async read: read without blocking. Return the line on success; otherwise
// register the fd with epoll.
struct ReadLine<'a> {
    reader: &'a mut AsyncReader,
}

impl<'a> Future for ReadLine<'a> {
    // return type
    type Output = Option<String>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut line = String::new();
        // async read
        match self.reader.reader.read_line(&mut line) {
            Ok(0) => Poll::Ready(None),        // connection closed
            Ok(_) => Poll::Ready(Some(line)), // read one line
            Err(err) => {
                // nothing to read (WouldBlock): register with epoll
                if err.kind() == std::io::ErrorKind::WouldBlock {
                    self.reader.selector.register(
                        EpollFlags::EPOLLIN,
                        self.reader.fd,
                        cx.waker().clone(),
                    );
                    Poll::Pending
                } else {
                    Poll::Ready(None)
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Task, Executor, Spawner (from 5.2.2 Scheduling)
// ---------------------------------------------------------------------------

struct Task {
    // coroutine to run
    future: Mutex<BoxFuture<'static, ()>>,
    // channel for scheduling onto the Executor
    sender: SyncSender<Arc<Task>>,
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        // schedule itself
        let self0 = arc_self.clone();
        arc_self.sender.send(self0).unwrap();
    }
}

struct Executor {
    // run queue
    sender: SyncSender<Arc<Task>>,
    receiver: Receiver<Arc<Task>>,
}

impl Executor {
    fn new() -> Self {
        // create the channel; the queue holds at most 1024 entries
        let (sender, receiver) = sync_channel(1024);
        Executor {
            sender: sender.clone(),
            receiver,
        }
    }

    // create a Spawner for creating new Tasks
    fn get_spawner(&self) -> Spawner {
        Spawner {
            sender: self.sender.clone(),
        }
    }

    fn run(&self) {
        // receive Tasks from the channel and run them in order
        while let Ok(task) = self.receiver.recv() {
            // create the context
            let mut future = task.future.lock().unwrap();
            let waker = waker_ref(&task);
            let mut ctx = Context::from_waker(&waker);
            // call poll to run it
            let _ = future.as_mut().poll(&mut ctx);
        }
    }
}

struct Spawner {
    sender: SyncSender<Arc<Task>>,
}

impl Spawner {
    fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        let future = future.boxed(); // box the Future
        let task = Arc::new(Task {
            // create the Task
            future: Mutex::new(future),
            sender: self.sender.clone(),
        });

        // enqueue onto the run queue
        self.sender.send(task).unwrap();
    }
}

// ---------------------------------------------------------------------------
// 5.3.2.3 Concurrent echo server with async/await
// ---------------------------------------------------------------------------

fn main() {
    let executor = Executor::new();
    let selector = IOSelector::new();
    let spawner = executor.get_spawner();

    // async code; the compiler generates an object implementing Future
    let server = async move {
        // create the listener for async accept (localhost port 10000)
        let listener = AsyncListener::listen("127.0.0.1:10000", selector.clone());
        loop {
            // accept a connection asynchronously
            let (mut reader, mut writer, addr) = listener.accept().await;
            println!("accept: {}", addr);

            // spawn a Task per connection
            spawner.spawn(async move {
                // per-connection work: read one line asynchronously and echo it back
                while let Some(buf) = reader.read_line().await {
                    print!("read: {}, {}", addr, buf);
                    writer.write(buf.as_bytes()).unwrap();
                    writer.flush().unwrap();
                }
                println!("close: {}", addr);
            });
        }
    };

    // spawn the task and run it
    executor.get_spawner().spawn(server);
    executor.run();
}

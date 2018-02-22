use std::thread;
use std::sync::{mpsc, Arc, Mutex};

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Job>,
}

impl ThreadPool {
    /// Create a new ThreadPool.
    ///
    /// The size is the number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero.
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool { workers, sender }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.sender.send(Box::new(f)).unwrap();
    }
}

struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                let job = receiver.lock().unwrap().recv().unwrap();
                println!("Worker {} got a job; executing.", id);
                job.call_box();
                println!("Worker {} done.", id);
            }
        });

        Worker { id, thread }
    }
}

// FnBox を使わずに Job を Box<FnOnce() + Send + 'static>
// として定義すると、 Box 内のクロージャをコールする際に
// コンパイルエラーになってしまう (`(*job)()`とは書けない)。
// これは、各クロージャは FnOnce trait を実装するそれぞれ別の型であり、
// Box 内のクロージャを move しようとしてもコンパイル時にそいつのサイズが
// 静的には決まらないため (たぶん)。
// そこで FnOnce trait を実装する全ての型に FnBox という trait を
// 実装し、`Box`内にいる場合のみ呼び出せる`call_box`を定義する。
// Generics により、`call_box`は実際にそれを使用するクロージャごとに実装されるため、
// コンパイル時に静的にサイズが決まる、という感じか (たぶん)。
// 面倒だし、将来的にはこういう tricky な処理は不要にしたい、との事。
type Job = Box<FnBox + Send + 'static>;

trait FnBox {
    fn call_box(self: Box<Self>);
}

impl<F: FnOnce()> FnBox for F {
    fn call_box(self: Box<F>) {
        (*self)();
    }
}

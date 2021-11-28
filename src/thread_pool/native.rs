use anyhow::Result;
use std::{
    cell::Cell,
    thread::{self, JoinHandle},
};

use super::ThreadPool;

pub struct NaiveThreadPool {
    workers: Cell<Vec<JoinHandle<()>>>,
}

impl ThreadPool for NaiveThreadPool {
    fn new(_threads: u32) -> Result<Self> {
        let workers = Cell::new(Vec::new());
        Ok(Self { workers })
    }

    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let mut workers = self.workers.take();
        workers.push(thread::spawn(job));
        self.workers.set(workers);
    }
}

impl NaiveThreadPool {
    #[allow(unused)]
    fn join(self) -> Result<()> {
        let workers = self.workers.take();
        for worker in workers {
            worker.join().unwrap();
        }
        Ok(())
    }
}

#[test]
fn test() {
    use std::thread::sleep;
    use std::time::Duration;
    let pool = NaiveThreadPool::new(4).unwrap();
    for i in 0..10 {
        pool.spawn(move || {
            println!("Hello from thread {}!", i);
            sleep(Duration::from_millis(2000));
        });
    }
    pool.join().unwrap();
}

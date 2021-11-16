use anyhow::Result;
use std::{
    thread::{self, sleep, JoinHandle},
    time::Duration,
};

pub struct ThreadNew {
    workers: Vec<JoinHandle<()>>,
}

impl ThreadNew {
    fn new(threads: u32) -> Result<Self> {
        let workers = Vec::new();
        Ok(Self {
            workers,
        })
    }

    fn spawn<F>(&mut self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.workers.push(thread::spawn(job));
    }

    fn join(self) -> thread::Result<()> {
        for worker in self.workers {
            worker.join()?;
        }
        Ok(())
    }
}

#[test]
fn test() {
    let mut pool = ThreadNew::new(4).unwrap();
    for i in 0..10 {
        pool.spawn(move || {
            println!("Hello from thread {}!", i);
            sleep(Duration::from_millis(2000));
        });
    }
    pool.join().unwrap();
}

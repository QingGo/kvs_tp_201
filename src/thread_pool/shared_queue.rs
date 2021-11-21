use anyhow::Result;
use crossbeam_channel::{bounded, Sender};
use std::thread::{self, JoinHandle};

use super::ThreadPool;

enum ThreadPoolMessage {
    RunJob(Box<dyn FnOnce() + Send + 'static>),
    Shutdown,
}
#[allow(unused)]
pub struct SharedQueueThreadPool {
    workers: Vec<JoinHandle<()>>,
    /// use crossbeam_channel for efficient communication
    sender: Sender<ThreadPoolMessage>,
}

impl ThreadPool for SharedQueueThreadPool {
    fn new(threads: u32) -> Result<Self> {
        let (sender, receiver) = bounded::<ThreadPoolMessage>(100);
        let mut workers = Vec::with_capacity(threads as usize);
        for _ in 0..threads {
            let receiver_clone = receiver.clone();
            let worker = thread::spawn(move || {
                while let Ok(task) = receiver_clone.recv() {
                    // the scope of the temporary value in the if let condition is the whole if/let construct
                    match task {
                        ThreadPoolMessage::RunJob(job) => {
                            job();
                        }
                        ThreadPoolMessage::Shutdown => {
                            break;
                        }
                    }
                }
            });
            workers.push(worker);
        }
        Ok(Self { workers, sender })
    }

    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.sender
            .send(ThreadPoolMessage::RunJob(Box::new(job)))
            .unwrap();
    }
}

impl SharedQueueThreadPool {
    #[allow(unused)]
    fn join(self) -> Result<()> {
        for i in 0..self.workers.len() {
            self.sender.send(ThreadPoolMessage::Shutdown).unwrap();
        }
        for worker in self.workers {
            worker.join().unwrap();
        }
        Ok(())
    }
}

#[test]
fn test() {
    use std::thread::sleep;
    use std::time::Duration;
    let pool = SharedQueueThreadPool::new(4).unwrap();
    for i in 0..10 {
        pool.spawn(move || {
            println!("Hello from thread {}!", i);
            sleep(Duration::from_millis(2000));
        });
    }
    pool.join().unwrap();
}

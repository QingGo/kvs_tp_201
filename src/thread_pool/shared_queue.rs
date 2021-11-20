use anyhow::Result;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{
    sync::Arc,
    thread::{self, sleep, JoinHandle},
    time::Duration,
};

use super::ThreadPool;

type ShardQueue = Arc<Mutex<VecDeque<Box<dyn FnOnce() + Send + 'static>>>>;

#[allow(unused)]
pub struct SharedQueueThreadPool {
    workers: Vec<JoinHandle<()>>,
    queue: ShardQueue,
    is_finish: Arc<AtomicBool>,
}

impl ThreadPool for SharedQueueThreadPool {
    fn new(threads: u32) -> Result<Self> {
        let queue: ShardQueue =
            Arc::new(Mutex::new(VecDeque::new()));
        let mut workers = Vec::with_capacity(threads as usize);
        let is_finish = Arc::new(AtomicBool::new(false));
        for _ in 0..threads {
            let queue_clone = Arc::clone(&queue);
            let is_finish_clone = Arc::clone(&is_finish);
            let worker = thread::spawn(move || loop {
                // the scope of the temporary value in the if let condition is the whole if/let construct
                let task_option = queue_clone.lock().unwrap().pop_front();
                if let Some(task) = task_option {
                    task();
                } else {
                    if is_finish_clone.load(Ordering::SeqCst) {
                        break;
                    }
                    sleep(Duration::from_millis(100));
                }
            });
            workers.push(worker);
        }
        Ok(Self {
            workers,
            queue,
            is_finish,
        })
    }

    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.queue.lock().unwrap().push_back(Box::new(job));
    }
}

impl SharedQueueThreadPool{
    #[allow(unused)]
    fn join(self) -> Result<()> {
        loop {
            if self.queue.lock().unwrap().len() == 0 {
                self.is_finish.store(true, Ordering::SeqCst);
                break;
            }
            sleep(Duration::from_millis(100));
        }
        for worker in self.workers {
            worker.join().unwrap();
        }
        Ok(())
    }
}

#[test]
fn test() {
    let pool = SharedQueueThreadPool::new(4).unwrap();
    for i in 0..10 {
        pool.spawn(move || {
            println!("Hello from thread {}!", i);
            sleep(Duration::from_millis(2000));
        });
    }
    pool.join().unwrap();
}
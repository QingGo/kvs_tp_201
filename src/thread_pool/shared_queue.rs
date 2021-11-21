use anyhow::Result;
use crossbeam_channel::{bounded, Sender};
use std::thread;

use super::ThreadPool;

enum ThreadPoolMessage {
    RunJob(Box<dyn FnOnce() + Send + 'static>),
    Shutdown,
}

#[allow(unused)]
pub struct SharedQueueThreadPool {
    num_threads: u32,
    /// use crossbeam_channel for efficient communication
    sender: Sender<ThreadPoolMessage>,
}

/// wrap crossbeam_channel::Receiver to imply drop trait
#[derive(Clone)]
struct TaskReceiver {
    receiver: crossbeam_channel::Receiver<ThreadPoolMessage>,
}

impl Drop for TaskReceiver {
    fn drop(&mut self) {
        // if thread end because of panic, then open a new thread run the task with the cloned receiver
        if thread::panicking() {
            let receiver = self.clone();
            thread::spawn(move || run_task(receiver));
        }
    }
}

impl ThreadPool for SharedQueueThreadPool {
    fn new(threads: u32) -> Result<Self> {
        log::info!("create shared queue thread pool with {} threads", threads);
        let (sender, receiver) = bounded::<ThreadPoolMessage>(100);
        for _ in 0..threads {
            let task_receiver = TaskReceiver {
                receiver: receiver.clone(),
            };
            thread::spawn(move || run_task(task_receiver));
        }
        Ok(Self {
            num_threads: threads,
            sender,
        })
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
        for i in 0..self.num_threads {
            self.sender.send(ThreadPoolMessage::Shutdown).unwrap();
        }
        Ok(())
    }
}

fn run_task(task_receiver: TaskReceiver) {
    while let Ok(task) = task_receiver.receiver.recv() {
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
}

#[test]
fn test() {
    use crossbeam_utils::sync::WaitGroup;
    use std::thread::sleep;
    use std::time::Duration;
    let pool = SharedQueueThreadPool::new(4).unwrap();
    let wg = WaitGroup::new();
    for i in 0..10 {
        let wg = wg.clone();
        pool.spawn(move || {
            println!("Hello from thread {}!", i);
            sleep(Duration::from_millis(2000));
            drop(wg);
        });
    }
    wg.wait();
}

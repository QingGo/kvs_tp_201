use super::ThreadPool;
use anyhow::Result;

pub struct RayonThreadPool {
    pool: rayon::ThreadPool,
}

impl ThreadPool for RayonThreadPool {
    fn new(threads: u32) -> Result<Self> {
        Ok(Self {
            pool: rayon::ThreadPoolBuilder::new()
                .num_threads(threads as usize)
                .build()?,
        })
    }
    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.pool.spawn(job);
    }
}

#[test]
fn test() {
    use std::thread::sleep;
    use std::time::Duration;
    let pool = RayonThreadPool::new(4).unwrap();
    for i in 0..10 {
        pool.spawn(move || {
            println!("Hello from thread {}!", i);
            sleep(Duration::from_millis(2000));
        });
    }
    sleep(Duration::from_millis(8000));
}



pub mod native;
pub mod shared_queue;
pub mod rayon;
pub use native::*;
pub use shared_queue::*;
pub use self::rayon::*;

use anyhow::Result;
pub trait ThreadPool {
    fn new(threads: u32) -> Result<Self>
    where
        Self: Sized;
    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static;
}

use super::store::KvStore;
use super::sled_engine::SledKvsEngine;

use super::error::Result;

pub trait KvsEngine: Send + 'static {
    fn new() -> Result<Self> where Self: Sized;
    fn clone(&self) -> Self where Self: Sized;
    fn set(&self, key: String, value: String) -> Result<()>;
    fn get(&self, key: String) -> Result<Option<String>>;
    fn remove(&self, key: String) -> Result<()>;
}

// surprising that it will cause cyclic-dependencies
pub fn get_engine_by_name(engine_name: &str) -> Box<dyn KvsEngine> {
    let engine: Box<dyn KvsEngine> = match engine_name {
        "kvs" => Box::new(KvStore::new().unwrap()),
        "sled" => Box::new(SledKvsEngine::new().unwrap()),
        _ => panic!("Unknown engine name"),
    };
    engine
}
use super::error::Result;

pub trait KvsEngine: Clone + Send + 'static {
    fn new() -> Result<Self>;
    fn set(&self, key: String, value: String) -> Result<()>;
    fn get(&self, key: String) -> Result<Option<String>>;
    fn remove(&self, key: String) -> Result<()>;
}

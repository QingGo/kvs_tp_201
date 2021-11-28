use super::error::Result;

pub trait KvsEngine: Send + 'static {
    fn new() -> Result<Self> where Self: Sized;
    fn clone(&self) -> Self where Self: Sized;
    fn set(&self, key: String, value: String) -> Result<()>;
    fn get(&self, key: String) -> Result<Option<String>>;
    fn remove(&self, key: String) -> Result<()>;
}

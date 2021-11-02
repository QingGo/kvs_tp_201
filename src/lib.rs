#![deny(missing_docs)]
//! A simple library for a simple KV in-memory database.

use std::collections::HashMap;

/// A simple KV in-memory database
/// ```rust
/// # use std::error::Error;
/// # use kvs::KvStore;
/// # fn main() -> Result<(), Box<dyn Error>> {
///     let mut db = KvStore::new();
///     db.set("key1".to_string(), "value1".to_string())?;
///     assert_eq!("value1", db.get("key1".to_string()).unwrap());
/// #   Ok(())
/// # }
/// ```
#[derive(Default)]
pub struct KvStore {
    data: HashMap<String, String>,
}

impl KvStore {
    /// get a new database instance
    pub fn new() -> KvStore {
        KvStore {
            data: HashMap::new(),
        }
    }
    /// set value for a key
    pub fn set(&mut self, key: String, value: String) -> Result<(), String> {
        self.data.insert(key, value);
        Ok(())
    }
    /// remove value for a key
    pub fn remove(&mut self, key: String) {
        self.data.remove(&key);
    }
    /// get value for a key
    pub fn get(&self, key: String) -> Option<String> {
        self.data.get(&key).map(|s| s.to_string())
    }
}

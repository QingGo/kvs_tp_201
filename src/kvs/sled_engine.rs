#![allow(dead_code)]
#![allow(unused_variables)]
use std::backtrace::Backtrace;

use crate::KvsError;

use super::engine::KvsEngine;
use super::error::Result;
pub struct SledKvsEngine {
    db: sled::Db,
}

impl KvsEngine for SledKvsEngine {
    fn set(&mut self, key: String, value: String) -> Result<()> {
        self.db.insert(key, &*value)?;
        Ok(())
    }

    fn get(&self, key: String) -> Result<Option<String>> {
        let result = self
            .db
            .get(key)?
            .map(|value| String::from_utf8_lossy(&value).to_string());
        self.db.flush()?;
        Ok(result)
    }

    fn remove(&mut self, key: String) -> Result<()> {
        self.db.remove(&key)?.ok_or(KvsError::KeyNotFound {
            key,
            backtrace: Backtrace::force_capture(),
        })?;
        self.db.flush()?;
        Ok(())
    }
}

impl SledKvsEngine {
    pub fn new() -> Result<Self> {
        let db = sled::open(".")?;
        Ok(SledKvsEngine { db })
    }
}

#![allow(dead_code)]
#![allow(unused_variables)]
use std::backtrace::Backtrace;
use std::path::PathBuf;

use crate::KvsError;

use super::engine::KvsEngine;
use super::error::Result;
#[derive(Clone)]
pub struct SledKvsEngine {
    db: sled::Db,
}

impl KvsEngine for SledKvsEngine {
    fn new() -> Result<Self> {
        let db = sled::open(".")?;
        Ok(SledKvsEngine { db })
    }

    fn set(&self, key: String, value: String) -> Result<()> {
        self.db.insert(key, &*value)?;
        self.db.flush()?;
        Ok(())
    }

    fn get(&self, key: String) -> Result<Option<String>> {
        let result = self
            .db
            .get(key)?
            .map(|value| String::from_utf8_lossy(&value).to_string());
        
        Ok(result)
    }

    fn remove(&self, key: String) -> Result<()> {
        self.db.remove(&key)?.ok_or(KvsError::KeyNotFound {
            key,
            backtrace: Backtrace::force_capture(),
        })?;
        self.db.flush()?;
        Ok(())
    }
}

impl SledKvsEngine {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let db = sled::open(path.into())?;
        Ok(SledKvsEngine { db })
    }
}

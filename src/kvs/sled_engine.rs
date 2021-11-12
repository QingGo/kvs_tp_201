#![allow(dead_code)]
#![allow(unused_variables)]
use super::engine::KvsEngine;
use super::error::Result;
pub struct SledKvsEngine {}

impl KvsEngine for SledKvsEngine {
    fn set(&mut self, key: String, value: String) -> Result<()> {
        unimplemented!()
    }

    fn get(&self, key: String) -> Result<Option<String>> {
        unimplemented!()
    }

    fn remove(&mut self, key: String) -> Result<()> {
        unimplemented!()
    }
}

impl SledKvsEngine {
    pub fn new() -> Self {
        SledKvsEngine {}
    }
}

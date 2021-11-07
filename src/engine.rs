#![deny(missing_docs)]
//! A simple library for a simple KV in-memory database.

extern crate thiserror;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::PathBuf;
use std::time;
use thiserror::Error;

/// A simple KV in-memory database. For readability, the commands are serialized by json, and using \n for separation.
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

pub struct KvStore {
    active_file_id: i64,
    dir: PathBuf,
    files: HashMap<PathBuf, File>,
    index: HashMap<String, Index>,
}

/// Result wrapper for KvsError
pub type Result<T> = std::result::Result<T, KvsError>;

/// Error type for KvStore
#[derive(Error, Debug)]
pub enum KvsError {
    /// IO Error type for KvStore
    #[error("io error")]
    Io(#[from] io::Error),
    /// Serde Error type for KvStore
    #[error("json serde error")]
    Serde(#[from] serde_json::Error),
    /// System time Error type for KvStore
    #[error("system time error")]
    SystemTimeError(#[from] time::SystemTimeError),
    /// Key not found Error type for KvStore
    #[error("key not found: {0})")]
    KeyNotFound(String),
    /// Unexpected command
    #[error("unexpected command: {0})")]
    UnexpectedCommand(String),
}

#[derive(Debug)]
struct Index {
    file_path: PathBuf,
    value_sz: usize,
    value_pos: u64,
    tstamp: u128,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
enum Command {
    Set,
    Get,
    Remove,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Record {
    command: Command,
    tstamp: u128,
    ksz: usize,
    value_sz: usize,
    key: String,
    value: String,
}

// 4 kb, for testing compatibility
const MAX_FILE_SIZE: u64 = 4 * 1024;

// TO-DO: Buffer and batch write
// maybe batch read is not necessary because the OS reads ~4kb block from disk into page cache every time
// need to perform reads from the log at arbitrary offsets. Consider how that might impact the way you manage file handles.
// compact the log file
impl KvStore {
    /// Open the KvStore at a given path. Return the KvStore.
    pub fn open(path: impl Into<PathBuf>) -> Result<KvStore> {
        let path_buf = path.into();
        fs::create_dir_all(&path_buf)?;
        let mut files = HashMap::new();
        let mut active_file_id = -1;
        for _file in fs::read_dir(&path_buf)? {
            let file = _file?;
            if file.file_type()?.is_file() {
                let file_path = file.path();
                if file_path.extension().unwrap_or(std::ffi::OsStr::new("")) == "db" {
                    let file_id_str = file_path.file_stem().unwrap().to_str().unwrap();
                    if !file_id_str.starts_with("compact_") {
                        let file_id = file_id_str.parse::<i64>().unwrap();
                        if file_id > active_file_id {
                            active_file_id = file_id;
                        }
                    }
                    let file_handle = fs::OpenOptions::new()
                        .write(true)
                        .create(true)
                        .read(true)
                        .open(&file_path)?;
                    files.entry(file_path).or_insert(file_handle);
                }
            }
        }
        // build index
        let index: HashMap<String, Index> = HashMap::new();
        let mut kvs = KvStore {
            active_file_id,
            dir: path_buf,
            files: HashMap::new(),
            index,
        };
        // do not move files into kvs now, or we can not borrow mut self double times
        for (file_path, file) in files.iter_mut() {
            kvs.build_index_from_file(file_path, file)?;
        }
        kvs.files = files;
        // if directory is empty, create a new file
        if kvs.files.is_empty() {
            kvs.generate_new_file(false)?;
        }
        Ok(kvs)
    }

    /// Set the value of a string key to a string. Return an error if the value is not written successfully.
    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        if self.get_last_file()?.metadata()?.len() > MAX_FILE_SIZE {
            self.generate_new_file(false)?;
            self.trigger_compaction()?;
        }
        let record = Record {
            command: Command::Set,
            tstamp: time::SystemTime::now()
                .duration_since(time::UNIX_EPOCH)?
                .as_micros(),
            ksz: key.len(),
            value_sz: value.len(),
            key: key.clone(),
            value,
        };
        let old_pos = self.get_last_file()?.seek(std::io::SeekFrom::End(0));
        let serialized = serde_json::to_vec(&record)?;
        {
            let mut writer = io::BufWriter::new(self.get_last_file()?);
            writer.write_all(&serialized)?;
            writer.write_all(b"\n")?;
        }
        self.index.insert(
            key,
            Index {
                file_path: self.dir.join(format!("{}.db", self.active_file_id)),
                value_sz: record.value.len(),
                value_pos: old_pos?,
                tstamp: time::SystemTime::now()
                    .duration_since(time::UNIX_EPOCH)?
                    .as_micros(),
            },
        );
        Ok(())
    }
    /// Get the string value of a string key. If the key does not exist, return None. Return an error if the value is not read successfully.
    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        let index = self.index.get(&key);
        match index {
            Some(index) => {
                let file = self.files.get_mut(&index.file_path).unwrap();
                file.seek(std::io::SeekFrom::Start(index.value_pos))?;
                let record: Record;
                let mut reader = io::BufReader::new(file);
                let mut line = String::new();
                reader.read_line(&mut line)?;
                record = serde_json::from_str(&line)?;
                Ok(Some(record.value))
            }
            None => Ok(None),
        }
    }
    /// Remove a given key. Return an error if the key does not exist or is not removed successfully.
    pub fn remove(&mut self, key: String) -> Result<()> {
        if self.get_last_file()?.metadata()?.len() > MAX_FILE_SIZE {
            self.generate_new_file(false)?;
            self.trigger_compaction()?;
        }
        self.index
            .remove(&key)
            .ok_or(KvsError::KeyNotFound(key.clone()))?;
        let record = Record {
            command: Command::Remove,
            tstamp: time::SystemTime::now()
                .duration_since(time::UNIX_EPOCH)?
                .as_micros(),
            ksz: key.len(),
            value_sz: 0,
            key,
            value: "".to_string(),
        };
        let serialized = serde_json::to_vec(&record)?;
        self.get_last_file()?.seek(io::SeekFrom::End(0))?;
        self.get_last_file()?.write_all(&serialized)?;
        Ok(())
    }

    fn get_last_file(&self) -> Result<&File> {
        Ok(&self
            .files
            .get(&self.dir.join(format!("{}.db", self.active_file_id)))
            .unwrap())
    }

    fn generate_new_file(&mut self, is_compact_file: bool) -> Result<&mut File> {
        let file_path;
        if is_compact_file {
            // compact file will be after at least one new normal file created
            // so two compaction will have different id
            file_path = self.dir.join(format!("compact_{}.db", self.active_file_id));
        } else {
            self.active_file_id += 1;
            file_path = self.dir.join(format!("{}.db", self.active_file_id));
        }
        let file_handle = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .read(true)
            .open(&file_path)?;
        self.files.insert(file_path.clone(), file_handle);
        Ok(self.files.get_mut(&file_path).unwrap())
    }

    fn build_index_from_file(&mut self, file_path: &PathBuf, file: &mut File) -> Result<()> {
        let mut reader = io::BufReader::new(file);
        loop {
            let old_pos = reader.seek(io::SeekFrom::Current(0))?;
            let mut buf = String::new();
            let n = reader.read_line(&mut buf)?;
            if n == 0 {
                break;
            }
            let record: Record = serde_json::from_str(&buf)?;
            self.build_index_from_record(&file_path, &record, old_pos)?;
        }
        Ok(())
    }

    fn build_index_from_record(
        &mut self,
        file_path: &PathBuf,
        record: &Record,
        old_pos: u64,
    ) -> Result<()> {
        match record.command {
            Command::Set => match self.index.entry(record.key.clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(Index {
                        file_path: file_path.clone(),
                        value_sz: record.value.len(),
                        value_pos: old_pos,
                        tstamp: record.tstamp,
                    });
                }
                Entry::Occupied(entry) => {
                    let entry_mut = entry.into_mut();
                    if entry_mut.tstamp <= record.tstamp{
                        entry_mut.tstamp = record.tstamp;
                        entry_mut.value_sz = record.value.len();
                        entry_mut.value_pos = old_pos;
                        entry_mut.file_path = file_path.clone();
                    }
                }
            },
            Command::Remove => {
                if let Some(e) = self.index.remove(&record.key) {
                    // when del record ts is less than the intex record ts,
                    // we should reinsert that record into the index
                    if e.tstamp >= record.tstamp {
                        self.index.insert(record.key.clone(), e);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn trigger_compaction(&mut self) -> Result<()> {
        let mut records: HashMap<String, Record> = HashMap::new();
        for (path, file) in self.files.iter_mut() {
            // skip the active file
            if *path == self.dir.join(format!("{}.db", self.active_file_id)) {
                continue;
            }
            // read all records from the inactive file
            file.seek(io::SeekFrom::Start(0))?;
            let mut reader = io::BufReader::new(file);
            loop {
                let mut buf = String::new();
                let n = reader.read_line(&mut buf)?;
                if n == 0 {
                    break;
                }
                let record: Record = serde_json::from_str(&buf)?;
                match record.command {
                    Command::Set => {
                        records
                            .entry(record.key.clone())
                            .and_modify(|e| {
                                if (*e).tstamp < record.tstamp {
                                    *e = record.clone();
                                }
                            })
                            .or_insert(record.clone());
                    }
                    Command::Remove => {
                        let key = record.key.clone();
                        if let Some(e) = records.remove(&key as &str) {
                            // when del record ts is less than the intex record ts,
                            // we should reinsert that record into the index
                            if e.tstamp > record.tstamp {
                                records.insert(key, record.clone());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        // write all records into the file
        let new_file = self.generate_new_file(true)?;
        let mut pos_vec = Vec::new();
        for record in records.values() {
            let serialized = serde_json::to_vec(&record)?;
            pos_vec.push(new_file.seek(io::SeekFrom::Current(0))?);
            new_file.write_all(&serialized)?;
            new_file.write_all(b"\n")?;
        }
        // can not use new_file and mut self same time, so split build inder logic here
        let file_path = self.dir.join(format!("compact_{}.db", self.active_file_id));
        for ((_, record), pos) in records.iter().zip(pos_vec.into_iter()) {
            self.build_index_from_record(&file_path, record, pos)?;
        }
        // remove old file
        let mut removed_path = Vec::new();
        for path in self.files.keys() {
            if *path != self.dir.join(format!("{}.db", self.active_file_id))
                && *path != self.dir.join(format!("compact_{}.db", self.active_file_id))
            {
                fs::remove_file(path)?;
                removed_path.push(path.clone());
            }
        }

        for path in removed_path {
            self.files.remove(&path);
        }
        Ok(())
    }
}

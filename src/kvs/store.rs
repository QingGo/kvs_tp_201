#![deny(missing_docs)]
//! A simple library for a simple KV in-memory database.
use super::error::Result;
use serde::{Deserialize, Serialize};
use std::backtrace::Backtrace;
use std::collections::{BTreeMap, HashMap};
use std::env::current_dir;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
// use std::sync::Arc;
use std::time;

use super::error::KvsError;

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
    db: Arc<RwLock<KvDB>>,
}

struct KvDB {
    active_file_id: u64,
    dir: PathBuf,
    file_handles: BTreeMap<u64, File>,
    indexes: HashMap<String, Index>,
    uncompacted_size: u64,
}

use super::engine::KvsEngine;

impl KvsEngine for KvStore {
    /// Open the KvStore at current path. Return the KvStore.
    fn new() -> Result<KvStore> {
        KvStore::open(current_dir()?)
    }

    /// Open the KvStore at a given path. Return the KvStore.
    fn open(path: impl Into<PathBuf>) -> Result<KvStore> {
        let dir = path.into();
        fs::create_dir_all(&dir)?;
        let db_file_ids = get_db_files_ids(&dir)?;
        let mut file_handles = get_file_handles(&dir, &db_file_ids)?;
        // build index
        let (indexes, uncompacted_size) = build_indexes(&mut file_handles)?;
        let active_file_id: u64;
        if db_file_ids.is_empty() {
            // create new file
            let file_handle = generate_new_file(&dir, 1)?;
            file_handles.insert(1, file_handle);
            active_file_id = 1;
        } else {
            active_file_id = *db_file_ids.last().unwrap();
        }
        let kv_db = KvDB {
            active_file_id,
            dir,
            file_handles,
            indexes,
            uncompacted_size,
        };
        Ok(KvStore {
            db: Arc::new(RwLock::new(kv_db)),
        })
    }

    /// Set the value of a string key to a string. Return an error if the value is not written successfully.
    fn set(&self, key: String, value: String) -> Result<()> {
        let record = Record {
            command: Command::Set,
            tstamp: time::SystemTime::now()
                .duration_since(time::UNIX_EPOCH)?
                .as_micros(),
            key,
            value,
        };
        self.clone().insert_record(record)?;
        Ok(())
    }
    /// Get the string value of a string key. If the key does not exist, return None. Return an error if the value is not read successfully.
    fn get(&self, key: String) -> Result<Option<String>> {
        let db = self.db.read().unwrap();
        let index = db.indexes.get(&key);
        match index {
            Some(index) => {
                let file = db.file_handles.get(&index.file_id).unwrap();
                // maybe because Seek trait have implemented for &File so this can work
                #[allow(clippy::clone_double_ref)]
                file.deref()
                    .clone()
                    .seek(std::io::SeekFrom::Start(index.value_pos))?;
                // with out take serde_json don't know how long to read
                let cmd_reader = file.deref().take(index.value_sz as u64);
                let record: Record = serde_json::from_reader(cmd_reader)?;
                Ok(Some(record.value))
            }
            None => Ok(None),
        }
    }
    /// Remove a given key. Return an error if the key does not exist or is not removed successfully.
    fn remove(&self, key: String) -> Result<()> {
        let record = Record {
            command: Command::Remove,
            tstamp: time::SystemTime::now()
                .duration_since(time::UNIX_EPOCH)?
                .as_micros(),
            key,
            value: "".to_string(),
        };
        self.clone().insert_record(record)?;
        Ok(())
    }

    fn clone(&self) -> Self {
        KvStore {
            db: self.db.clone(),
        }
    }
}

#[derive(Debug, Clone)]
struct Index {
    file_id: u64,
    value_sz: u64,
    value_pos: u64,
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
    key: String,
    value: String,
}

// 4 kb, for testing compatibility
// const MAX_FILE_SIZE: u64 = 4 * 1024;
const TRIGGER_COMPACT_SIZE: u64 = 4 * 1024;

// TO-DO: Buffer and batch write
// batch read using BufReader is necessary because although the OS reads ~4kb block from disk into page cache every time
// but read is a system call which is realtively expensive
// need to perform reads from the log at arbitrary offsets. Consider how that might impact the way you manage file handles.
// compact the log file
impl KvStore {
    fn insert_record(&mut self, record: Record) -> Result<()> {
        let mut db = self.db.write().unwrap();
        let mut active_file = get_last_file(&db.file_handles, db.active_file_id)?;
        let old_pos = active_file.seek(std::io::SeekFrom::End(0))?;
        serde_json::to_writer(active_file, &record)?;
        let new_pos = active_file.seek(std::io::SeekFrom::End(0))?;
        let active_file_id = db.active_file_id;
        match record.command {
            Command::Set => {
                if let Some(old_index) = db.indexes.insert(
                    record.key,
                    Index {
                        file_id: active_file_id,
                        value_sz: new_pos - old_pos,
                        value_pos: old_pos,
                    },
                ) {
                    db.uncompacted_size += old_index.value_sz;
                }
            }
            Command::Remove => {
                if let Some(old_index) = db.indexes.remove(&record.key) {
                    db.uncompacted_size += old_index.value_sz;
                } else {
                    return Err(KvsError::KeyNotFound {
                        key: record.key,
                        backtrace: Backtrace::force_capture(),
                    });
                }
            }
            _ => {}
        }

        if db.uncompacted_size > TRIGGER_COMPACT_SIZE {
            self.compact(&mut db)?;
        }
        Ok(())
    }

    fn compact(&self, db: &mut KvDB) -> Result<()> {
        // should not try to get lock here cause function insert_record has get the lock and it will be block forever
        // let mut db = self.db.lock().unwrap();
        let mut compact_file = generate_new_file(&db.dir, db.active_file_id + 1)?;
        // compact all include current active file
        // use index to find the record
        let mut active_file_id = db.active_file_id;
        // when need to change two or more fields in the same time(such as loop), replace/take/swap or RefCell
        let mut file_handles = std::mem::take(&mut db.file_handles);
        let indexes = db.indexes.iter_mut();
        let mut pos = 0;
        for (_, index) in indexes {
            let file = file_handles.get_mut(&index.file_id).unwrap();
            file.seek(std::io::SeekFrom::Start(index.value_pos))?;
            // tricky, use io::copy to copy the record
            let mut entry_reader = file.take(index.value_sz);
            io::copy(&mut entry_reader, &mut compact_file)?;
            // update index
            index.file_id = active_file_id + 1;
            index.value_pos = pos;
            pos = compact_file.seek(std::io::SeekFrom::End(0))?;
        }
        std::mem::swap(&mut db.file_handles, &mut file_handles);
        // remove old file, can not remove during loop
        let file_ids: Vec<u64> = db.file_handles.keys().cloned().collect();
        for file_id in file_ids {
            db.file_handles.remove(&file_id);
            fs::remove_file(db.dir.join(format!("{}.db", file_id)))?;
        }
        // update file handle and active file id, should be lock if compact by another thread?
        active_file_id += 1;
        db.file_handles.insert(active_file_id, compact_file);
        active_file_id += 1;
        let active_file = generate_new_file(&db.dir, active_file_id)?;
        db.file_handles.insert(active_file_id, active_file);
        db.active_file_id = active_file_id;

        Ok(())
    }
}

fn get_db_files_ids(dir: &Path) -> Result<Vec<u64>> {
    let mut files_ids = fs::read_dir(&dir)?
        .flat_map(|entry| -> Result<_> { Ok(entry?.path()) })
        .filter(|path| path.is_file() && path.extension().unwrap_or_default() == "db")
        .flat_map(|path| {
            path.file_stem()
                .and_then(OsStr::to_str)
                .map(str::parse::<u64>)
        })
        .flatten()
        .collect::<Vec<u64>>();
    files_ids.sort_unstable();
    Ok(files_ids)
}

fn get_file_handles(dir: &Path, file_ids: &[u64]) -> Result<BTreeMap<u64, File>> {
    let mut handles = BTreeMap::new();
    for file_id in file_ids {
        handles.insert(*file_id, generate_new_file(dir, *file_id)?);
    }
    Ok(handles)
}

fn build_indexes(file_handles: &mut BTreeMap<u64, File>) -> Result<(HashMap<String, Index>, u64)> {
    let mut indexes = HashMap::new();
    let mut uncompacted_size: u64 = 0;
    // loop all file in order instead of using timestamp to choose new record to build index
    for (file_id, file) in file_handles.iter() {
        let mut pos = 0;
        // maybe use buffer will be better,
        let mut records_stream =
            serde_json::Deserializer::from_reader(file.deref()).into_iter::<Record>();
        // tricky, if use for record in records_stream,
        // records_stream.byte_offset() will not work because records_stream has been moved
        while let Some(record) = records_stream.next() {
            let record = record?;
            // get offset from stream instead of file
            let new_pos = records_stream.byte_offset();
            match record.command {
                Command::Set => {
                    if let Some(old_index) = indexes.insert(
                        record.key,
                        Index {
                            file_id: *file_id,
                            value_sz: new_pos as u64 - pos,
                            value_pos: pos,
                        },
                    ) {
                        uncompacted_size += old_index.value_sz;
                    }
                }
                Command::Remove => {
                    if let Some(old_index) = indexes.remove(&record.key) {
                        uncompacted_size += old_index.value_sz;
                    }
                }
                _ => {}
            }
            pos = new_pos as u64;
        }
    }
    Ok((indexes, uncompacted_size))
}

fn generate_new_file(path: &Path, file_id: u64) -> Result<File> {
    let file_path = path.join(format!("{}.db", file_id));
    let file_handle = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .read(true)
        .open(&file_path)?;
    Ok(file_handle)
}

fn get_last_file(file_handles: &BTreeMap<u64, File>, active_file_id: u64) -> Result<&File> {
    Ok(file_handles.get(&active_file_id).unwrap())
}

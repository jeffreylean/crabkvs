use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    ffi::OsStr,
    fmt::Debug,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Seek, SeekFrom, Write},
    path::{self, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
pub mod error;

// Maximum size of a log file should be 2kb
const MAX_SIZE: u64 = 4096;

const COMPACTION_THRESHOLD: u64 = 50 * 1024;

/// This is a type that contain hashmap which is used as a memory storage
pub struct KvStore {
    writer: BufWriter<File>,
    keydir: HashMap<String, Option<MetaData>>,
    current_split: u64,
    active_log: PathBuf,
    path: PathBuf,
    total_log_size: u64,
}
struct MetaData {
    reader: BufReader<File>,
    pos: u64,
    timestamp: u128,
}

#[derive(Serialize, Deserialize, Debug)]
enum Command {
    Set {
        key: String,
        value: String,
        timestamp: u128,
    },
    Get {
        key: String,
    },
    Remove {
        key: String,
        value: String,
        timestamp: u128,
    },
}

impl Default for KvStore {
    fn default() -> Self {
        Self::open(path::Path::new("log")).expect("failed to open log file")
    }
}

/// Methods of type KvStore which consists of usual key value store operation like get, set and
// remove
impl KvStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<KvStore> {
        let log_file_dir: PathBuf = path.into();
        // Get list of log files numbers
        let list = sorted_split_list(&log_file_dir)?;
        let index = if list.is_empty() {
            0
        } else {
            *list.last().unwrap()
        };
        let log_file_path = log_file_dir.join(format!("{}.log", index));

        let write_file = OpenOptions::new()
            .create(true)
            .append(true)
            .write(true)
            .open(&log_file_path)?;

        let mut kvs = KvStore {
            keydir: HashMap::new(),
            writer: BufWriter::new(write_file),
            current_split: index,
            active_log: log_file_path.clone(),
            total_log_size: 0,
            path: log_file_dir.clone(),
        };

        for split in list.iter() {
            let mut buf = String::new();
            let file = &log_file_dir.join(format!("{}.log", split));
            let mut offset = 0;
            let mut reader = BufReader::new(File::open(file)?);
            while reader.read_line(&mut buf)? > 0 {
                match serde_json::from_str(&buf)? {
                    Command::Set { key, timestamp, .. } => {
                        let _ = kvs.keydir.insert(
                            key,
                            Some(MetaData {
                                pos: offset,
                                reader: BufReader::new(File::open(file)?),
                                timestamp,
                            }),
                        );
                        kvs.total_log_size += buf.as_bytes().len() as u64;
                    }
                    Command::Remove { key, .. } => {
                        let _ = kvs.keydir.remove(&key);
                    }
                    // Not sure why here i am forced to return Option<u16>, which is the return type of
                    // previous 2 enum.
                    // Does the return type have to be the same for all enum ???
                    Command::Get { .. } => (),
                };

                offset = reader.stream_position()?;
                // Update the cursor of the writer so that it will end up at the lastest write
                // offset
                kvs.writer.seek(SeekFrom::Start(offset))?;
                buf.clear();
            }
        }
        Ok(kvs)
    }
    /// Get data based on given key
    ///
    /// # Arguments
    ///
    /// * `key` - A string that holds the key
    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        if let Some(Some(meta)) = self.keydir.get_mut(&key) {
            let mut buf = String::new();
            // go to the line offset
            meta.reader.seek(SeekFrom::Start(meta.pos))?;
            // read the line
            meta.reader.read_line(&mut buf)?;
            if let Ok(Command::Set { value, .. }) = serde_json::from_str(&buf) {
                return Ok(Some(value));
            } else {
                return Err(anyhow!("incorrect format log entry"));
            }
        }
        Ok(None)
    }
    /// Create key-value record entry
    ///
    /// # Arguments
    ///
    /// * `key` - A string that holds the key
    /// * `value` - A string that holds the value to store
    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
        let offset = self.write_to_log(&Command::Set {
            key: key.clone(),
            value,
            timestamp: current_time,
        })?;
        let reader = BufReader::new(File::open(&self.active_log)?);
        self.keydir.insert(
            key,
            Some(MetaData {
                reader,
                pos: offset,
                timestamp: current_time,
            }),
        );

        // Check if compaction is needed
        if self.total_log_size > COMPACTION_THRESHOLD {
            self.compaction()?;
        }

        Ok(())
    }
    /// Remove data based on given key
    ///
    /// # Arguments
    ///
    /// * `key` - A string that holds the key to the targeted entry
    pub fn remove(&mut self, key: String) -> Result<()> {
        let mut buf = String::new();
        if let Some(Some(meta)) = self.keydir.get_mut(&key) {
            meta.reader.seek(SeekFrom::Start(meta.pos))?;
            meta.reader.read_line(&mut buf)?;
            if let Ok(Command::Set { key, value, .. }) = serde_json::from_str(&buf) {
                self.write_to_log(&Command::Remove {
                    key: key.clone(),
                    value,
                    timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
                })?;
                self.keydir.insert(key, None);
                // Check if compaction is needed
                if self.total_log_size > COMPACTION_THRESHOLD {
                    self.compaction()?;
                }
            }
            return Ok(());
        }
        bail!(error::Error::KeyNotFound)
    }

    fn write_to_log<T: Sized + Serialize>(&mut self, entry: &T) -> Result<u64> {
        let entry_size = serde_json::to_string(entry)?.len() as u64;
        let active_log_file_size = self.writer.get_ref().metadata().unwrap().len();
        // Update total log size with this new entry size
        self.total_log_size += entry_size;
        // If file size threshold is reached for this new log entry
        if entry_size + active_log_file_size > MAX_SIZE {
            let offset = self.writer.stream_position()?;
            self.current_split += 1;
            // Create new active file
            let path = self.path.join(format!("{}.log", self.current_split));
            let writer = OpenOptions::new()
                .create(true)
                .append(true)
                .write(true)
                .open(&path)?;

            self.writer = BufWriter::new(writer);
            self.active_log = path;

            let log = serde_json::to_string(entry).map(|mut s| {
                s.push('\n');
                s
            })?;
            self.writer.write_all(log.as_bytes())?;
            self.writer.flush()?;
            Ok(offset)
        } else {
            let offset = self.writer.stream_position()?;
            let log = serde_json::to_string(entry).map(|mut s| {
                s.push('\n');
                s
            })?;
            self.writer.write_all(log.as_bytes())?;
            self.writer.flush()?;
            Ok(offset)
        }
    }

    fn compaction(&mut self) -> Result<()> {
        // List out all the log file index
        let list = sorted_split_list(&self.path)?;
        // Create new log file after the latest log file
        let compact_writer = OpenOptions::new()
            .create(true)
            .append(true)
            .write(true)
            .open(self.path.join(format!("{}.log", list.last().unwrap() + 1)))?;
        // Update the current writer
        self.writer = BufWriter::new(compact_writer);
        // Reset log size counter
        self.total_log_size = 0;

        for idx in &list {
            let mut reader = BufReader::new(File::open(self.path.join(format!("{}.log", idx)))?);
            let mut buff = String::new();
            while reader.read_line(&mut buff)? > 0 {
                match serde_json::from_str(&buff)? {
                    Command::Set {
                        key,
                        value,
                        timestamp,
                        ..
                    } => {
                        if let Some(Some(meta)) = self.keydir.get_mut(&key) {
                            if meta.timestamp == timestamp {
                                self.write_to_log(&Command::Set {
                                    key,
                                    value,
                                    timestamp,
                                })?;
                            }
                        }
                    }
                    Command::Remove { .. } => {}
                    Command::Get { .. } => {}
                }
                buff.clear();
            }
            drop(reader);
        }

        // Delete stale log file
        for idx in list {
            fs::remove_file(&self.path.join(format!("{}.log", idx)))?;
        }

        Ok(())
    }
}

fn sorted_split_list(path: &Path) -> Result<Vec<u64>> {
    let mut list: Vec<u64> = fs::read_dir(path)?
        .flat_map(|res| -> Result<_> { Ok(res?.path()) })
        .filter(|path| path.is_file() && path.extension() == Some("log".as_ref()))
        .flat_map(|path| {
            path.file_name()
                .and_then(OsStr::to_str)
                .map(|s| s.trim_end_matches(".log"))
                .map(str::parse::<u64>)
        })
        .flatten()
        .collect();

    list.sort_unstable();
    Ok(list)
}

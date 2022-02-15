use serde_json::Deserializer;

use crate::{commands::CommandPosition, kvs_error::Result, Command, KvStoreError};
use std::{
    collections::BTreeMap,
    env::current_dir,
    fs::{File, OpenOptions},
    io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

/// The `KvStore` stores string key/value pairs.
///
/// Key/value pairs are stored in a `HashMap` in memory and not persisted to disk.
///
/// Example:
///
/// ```rust
/// # use kvs::KvStore;
/// let mut store = KvStore::new();
/// store.set("key".to_owned(), "value".to_owned());
/// let val = store.get("key".to_owned());
/// assert_eq!(val, Some("value".to_owned()));
/// ```
#[derive(Debug)]
pub struct KvStore {
    pub path: PathBuf,
    pub writer: BufWriterWithPos<File>,
    reader: BufReaderWithPos<File>,
    index: BTreeMap<String, CommandPosition>,
}

impl KvStore {
    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        let command = Command::Set {
            key: key.clone(),
            value,
        };

        let curr_position = self.writer.position;
        serde_json::to_writer(&mut self.writer, &command)?;
        self.writer.flush()?;
        self.index.insert(
            key,
            CommandPosition {
                start: curr_position,
                length: self.writer.position,
            },
        );

        Ok(())
    }

    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        if let Some(cmd_position) = self.index.get(&key) {
            let reader = self.reader.source.get_mut();
            reader
                .seek(SeekFrom::Start(cmd_position.start))
                .expect("Couldn't get mutable reference to reader");
            let taken = reader.take(cmd_position.length);
            if let Command::Set { value, key: _ } = serde_json::from_reader(taken)? {
                return Ok(Some(value));
            } else {
                return Err(KvStoreError::InvalidLogFileCommand);
            }
        } else {
            return Ok(None);
        }
    }

    pub fn remove(&mut self, key: String) -> Result<()> {
        if let Some(_) = self.index.remove(&key) {
            let command = Command::Rm { key };
            serde_json::to_writer(&mut self.writer, &command)?;
            self.writer.flush()?;
            Ok(())
        } else {
            return Err(KvStoreError::KeyNotFound);
        }
    }

    pub fn open(path: impl Into<PathBuf>) -> Result<KvStore> {
        let mut path: PathBuf = path.into();
        if let Some(_path) = path.to_str() {
            if _path.is_empty() {
                path = current_dir()?;
            }
        }

        if path.is_dir() {
            path.push("default_log_file.txt");
        }

        let mut writer = BufWriterWithPos::new(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(path.clone())?,
        );

        let mut index = BTreeMap::new();

        let mut reader = BufReaderWithPos::new(File::open(path.clone())?);
        let reader_clone = reader.source.get_mut();
        let mut initial_pos = reader_clone.seek(SeekFrom::Start(0))?;
        let mut stream = Deserializer::from_reader(reader_clone).into_iter::<Command>();
        while let Some(cmd) = stream.next() {
            let offset = stream.byte_offset() as u64;
            match cmd? {
                Command::Set { key, value: _ } => {
                    index.insert(
                        key,
                        CommandPosition {
                            start: initial_pos,
                            length: offset,
                        },
                    );
                }
                Command::Rm { key } => {
                    index.remove(&key);
                }
                _ => {}
            }
            initial_pos = offset;
        }
        writer.position = initial_pos;

        let reader = BufReaderWithPos::new(File::open(path.clone())?);

        Ok(KvStore {
            path,
            reader,
            writer,
            index,
        })
    }
}

#[derive(Debug)]
pub struct BufWriterWithPos<T: Write + Seek> {
    source: BufWriter<T>,
    pub position: u64,
}

impl<T: Write + Seek> BufWriterWithPos<T> {
    pub fn new(source: T) -> Self {
        Self {
            source: BufWriter::new(source),
            position: 0,
        }
    }
}

#[derive(Debug)]
pub struct BufReaderWithPos<T: Read + Seek> {
    source: BufReader<T>,
    position: u64,
}

impl<T: Read + Seek> BufReaderWithPos<T> {
    pub fn new(source: T) -> Self {
        Self {
            source: BufReader::new(source),
            position: 0,
        }
    }
}

impl<T: Write + Seek> Write for BufWriterWithPos<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let bytes_written = self.source.write(buf)?;
        self.position += bytes_written as u64;
        Ok(bytes_written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.source.flush()
    }
}

impl<T: Read + Seek> Read for BufReaderWithPos<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let read = self.source.read(buf)?;
        self.position += read as u64;
        Ok(read)
    }
}

impl<T: Read + Seek> Seek for BufReaderWithPos<T> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.position = self.source.seek(pos)?;
        Ok(self.position)
    }
}

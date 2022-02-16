use serde_json::Deserializer;

use crate::{commands::CommandPosition, kvs_error::Result, Command, KvStoreError};
use std::{
    collections::BTreeMap,
    env::current_dir,
    fs::{self, File, OpenOptions},
    io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

const THRESHOLD: u64 = 8008135;

/// The `KvStore` stores string key/value pairs.
///
/// Key/value pairs are stored in a `HashMap` in memory and not persisted to disk.
///
/// Example:
///
/// ```rust
/// # use kvs::KvStore;
/// let mut store = KvStore::open("").unwrap();
/// store.set("key".to_owned(), "value".to_owned()).unwrap();
/// let val = store.get("key".to_owned()).unwrap();
/// assert_eq!(val, Some("value".to_owned()));
/// ```
#[derive(Debug)]
pub struct KvStore {
    pub path: PathBuf,
    pub writer: BufWriterWithPos<File>,
    reader: BufReaderWithPos<File>,
    pub index: BTreeMap<String, CommandPosition>,
    dirt: u64,
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
        if let Some(old_value) = self.index.insert(
            key,
            CommandPosition {
                start: curr_position,
                length: self.writer.position - curr_position,
            },
        ) {
            self.dirt += old_value.length;
        }

        if self.dirt >= THRESHOLD {
            self.compact()?;
            self.dirt = 0;
        }

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
                            length: offset - initial_pos,
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
            dirt: 0,
        })
    }

    fn compact(&mut self) -> Result<()> {
        let mut curr_position = 0;
        let mut new_values = vec![];

        for cmds in self.index.values_mut() {
            if self.reader.position != cmds.start {
                self.reader.seek(SeekFrom::Start(cmds.start))?;
            }
            let reader = self.reader.source.get_ref();
            let taken = reader.take(cmds.length);

            if let Command::Set { value, key } = serde_json::from_reader(taken)? {
                cmds.start = curr_position;
                curr_position += cmds.length;
                new_values.push(Command::Set {
                    key: key.clone(),
                    value,
                });
            }
        }

        fs::remove_file(&self.path)?;
        self.writer = BufWriterWithPos::new(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?,
        );

        self.reader = BufReaderWithPos::new(File::open(&self.path)?);
        for cmd in new_values {
            serde_json::to_writer(&mut self.writer, &cmd)?;
        }
        self.writer.flush()?;
        self.reader.seek(SeekFrom::Start(0))?;
        self.writer.seek(SeekFrom::Start(0))?;

        Ok(())
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

impl<T: Write + Seek> Seek for BufWriterWithPos<T> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.position = self.source.seek(pos)?;
        Ok(self.position)
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

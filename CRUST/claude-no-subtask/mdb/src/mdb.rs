use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
const DB_NAME_MAX: usize = 128; // Assuming a reasonable maximum length
type MdbPtr = u32;
type MdbSize = u32;
const MDB_PTR_SIZE: usize = std::mem::size_of::<MdbPtr>();
const MDB_DATALEN_SIZE: usize = std::mem::size_of::<MdbSize>();
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MdbStatusCode {
    MDB_OK = 0,
    MDB_NO_KEY,
    MDB_ERR_CRITICAL,
    MDB_ERR_LOGIC,
    MDB_ERR_FLUSH,
    MDB_ERR_OPEN_FILE,
    MDB_ERR_READ,
    MDB_ERR_WRITE,
    MDB_ERR_ALLOC,
    MDB_ERR_SEEK,
    MDB_ERR_BUFSIZ,
    MDB_ERR_KEY_SIZE,
    MDB_ERR_VALUE_SIZE,
    MDB_ERR_UNIMPLEMENTED = 100,
}
#[derive(Debug, Clone)]
pub struct MdbOptions {
    pub db_name: String,
    pub key_size_max: u16,
    pub data_size_max: u32,
    pub hash_buckets: u32,
    pub items_max: u32,
}
#[derive(Debug)]
pub struct MdbStatus{
    pub code: u8,
    pub desc: String,
}
#[derive(Debug)]
pub enum MdbError {
    Io(io::Error),
    AllocationFailed,
    BufferSizeTooSmall,
    KeyNotFound,
    KeySizeTooLarge,
    ValueSizeTooLarge,
}
impl From<io::Error> for MdbError {
    fn from(error: io::Error) -> Self {
        MdbError::Io(error)
    }
}
pub type Result<T> = std::result::Result<T, MdbError>;
struct MdbIndex {
    next_ptr: MdbPtr,
    value_ptr: MdbPtr,
    value_size: MdbSize,
    key: Vec<u8>,
}
pub struct Mdb {
    db_name: String,
    fp_superblock: File,
    fp_index: File,
    fp_data: File,
    options: MdbOptions,
    index_record_size: u32,
}

fn invalid_data(msg: &'static str) -> MdbError {
    MdbError::Io(io::Error::new(io::ErrorKind::InvalidData, msg))
}

fn build_paths<P: AsRef<Path>>(path: P) -> (PathBuf, PathBuf, PathBuf) {
    let p = path.as_ref();
    let s = p.to_string_lossy().into_owned();
    (
        PathBuf::from(format!("{}.super", s)),
        PathBuf::from(format!("{}.index", s)),
        PathBuf::from(format!("{}.data", s)),
    )
}

impl Mdb {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let (super_path, index_path, data_path) = build_paths(&path);

        // Open superblock for reading and parse it.
        let f = File::open(&super_path)?;
        let reader = BufReader::new(f);
        let mut lines_iter = reader.lines();

        let db_name = lines_iter
            .next()
            .ok_or_else(|| invalid_data("missing db_name in superblock"))??;
        let key_size_max: u16 = lines_iter
            .next()
            .ok_or_else(|| invalid_data("missing key_size_max"))??
            .parse()
            .map_err(|_| invalid_data("invalid key_size_max"))?;
        let data_size_max: u32 = lines_iter
            .next()
            .ok_or_else(|| invalid_data("missing data_size_max"))??
            .parse()
            .map_err(|_| invalid_data("invalid data_size_max"))?;
        let hash_buckets: u32 = lines_iter
            .next()
            .ok_or_else(|| invalid_data("missing hash_buckets"))??
            .parse()
            .map_err(|_| invalid_data("invalid hash_buckets"))?;
        let items_max: u32 = lines_iter
            .next()
            .ok_or_else(|| invalid_data("missing items_max"))??
            .parse()
            .map_err(|_| invalid_data("invalid items_max"))?;

        let options = MdbOptions {
            db_name: db_name.clone(),
            key_size_max,
            data_size_max,
            hash_buckets,
            items_max,
        };

        let index_record_size =
            key_size_max as u32 + (MDB_PTR_SIZE as u32) * 2 + MDB_DATALEN_SIZE as u32;

        // Reopen superblock to retain a handle in the struct (matches C semantics).
        let fp_superblock = File::open(&super_path)?;
        let fp_index = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&index_path)?;
        let fp_data = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&data_path)?;

        Ok(Mdb {
            db_name,
            fp_superblock,
            fp_index,
            fp_data,
            options,
            index_record_size,
        })
    }

    pub fn create<P: AsRef<Path>>(path: P, options: MdbOptions) -> Result<Self> {
        let (super_path, index_path, data_path) = build_paths(&path);

        let index_record_size =
            options.key_size_max as u32 + (MDB_PTR_SIZE as u32) * 2 + MDB_DATALEN_SIZE as u32;

        // Write the superblock.
        let mut fp_superblock = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&super_path)?;
        writeln!(fp_superblock, "{}", options.db_name)?;
        writeln!(fp_superblock, "{}", options.key_size_max)?;
        writeln!(fp_superblock, "{}", options.data_size_max)?;
        writeln!(fp_superblock, "{}", options.hash_buckets)?;
        writeln!(fp_superblock, "{}", options.items_max)?;
        fp_superblock.flush()?;

        // Initialize the index file with the freeptr (0) followed by hash_buckets
        // bucket pointers (all 0).
        let mut fp_index = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&index_path)?;
        let total_ptrs = 1usize + options.hash_buckets as usize;
        let zeros = vec![0u8; total_ptrs * MDB_PTR_SIZE];
        fp_index.write_all(&zeros)?;
        fp_index.flush()?;

        // Create an empty data file.
        let fp_data = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&data_path)?;

        let db_name = options.db_name.clone();
        Ok(Mdb {
            db_name,
            fp_superblock,
            fp_index,
            fp_data,
            options,
            index_record_size,
        })
    }

    pub fn read(&mut self, key: &str, buf: &mut [u8]) -> Result<usize> {
        let bucket = self.hash(key) % self.options.hash_buckets;
        let mut ptr = self.read_bucket(bucket)?;

        while ptr != 0 {
            let index = self.read_index(ptr)?;
            if Self::key_matches(&index.key, key.as_bytes()) {
                return self.read_data(index.value_ptr, index.value_size, buf);
            }
            ptr = index.next_ptr;
        }

        Err(MdbError::KeyNotFound)
    }

    pub fn write(&mut self, key: &str, value: &str) -> Result<()> {
        let bucket = self.hash(key) % self.options.hash_buckets;
        let key_size = key.len();
        if key_size > self.options.key_size_max as usize {
            return Err(MdbError::KeySizeTooLarge);
        }
        let value_size = value.len() as u32;
        if value_size > self.options.data_size_max {
            return Err(MdbError::ValueSizeTooLarge);
        }

        let mut save_ptr: MdbPtr = (MDB_PTR_SIZE as u32) * (bucket + 1);
        let mut ptr = self.read_bucket(bucket)?;
        let mut found_index: Option<MdbIndex> = None;

        while ptr != 0 {
            let index = self.read_index(ptr)?;
            if Self::key_matches(&index.key, key.as_bytes()) {
                found_index = Some(index);
                break;
            }
            save_ptr = ptr;
            ptr = index.next_ptr;
        }

        if ptr == 0 {
            let mut index_ptr: MdbPtr = 0;
            self.index_alloc(&mut index_ptr)?;
            let mut value_ptr: MdbPtr = 0;
            if let Err(e) = self.data_alloc(value_size, &mut value_ptr) {
                let _ = self.index_free(index_ptr);
                return Err(e);
            }
            if let Err(e) = self.write_data(value_ptr, value.as_bytes(), value_size) {
                let _ = self.data_free(value_ptr, value_size);
                let _ = self.index_free(index_ptr);
                return Err(e);
            }
            if let Err(e) =
                self.write_index(index_ptr, key.as_bytes(), value_ptr, value_size)
            {
                let _ = self.data_free(value_ptr, value_size);
                let _ = self.index_free(index_ptr);
                return Err(e);
            }
            self.write_nextptr(save_ptr, index_ptr)?;
            Ok(())
        } else {
            let index = found_index.unwrap();
            self.data_free(index.value_ptr, index.value_size)?;
            let mut value_ptr: MdbPtr = 0;
            self.data_alloc(value_size, &mut value_ptr)?;
            self.write_data(value_ptr, value.as_bytes(), value_size)?;
            self.write_index(ptr, key.as_bytes(), value_ptr, value_size)?;
            Ok(())
        }
    }

    pub fn delete(&mut self, key: &str) -> Result<()> {
        let bucket = self.hash(key) % self.options.hash_buckets;
        let mut save_ptr: MdbPtr = (MDB_PTR_SIZE as u32) * (bucket + 1);
        let mut ptr = self.read_bucket(bucket)?;
        let mut found_index: Option<MdbIndex> = None;

        while ptr != 0 {
            let index = self.read_index(ptr)?;
            if Self::key_matches(&index.key, key.as_bytes()) {
                found_index = Some(index);
                break;
            }
            save_ptr = ptr;
            ptr = index.next_ptr;
        }

        if ptr == 0 {
            return Err(MdbError::KeyNotFound);
        }

        let index = found_index.unwrap();
        self.data_free(index.value_ptr, index.value_size)?;
        self.index_free(ptr)?;
        self.write_nextptr(save_ptr, index.next_ptr)?;
        Ok(())
    }

    pub fn get_options(&self) -> &MdbOptions {
        &self.options
    }

    pub fn index_size(&mut self) -> Result<u64> {
        Ok(self.fp_index.seek(SeekFrom::End(0))?)
    }

    pub fn data_size(&mut self) -> Result<u64> {
        Ok(self.fp_data.seek(SeekFrom::End(0))?)
    }

    // Private helper methods
    fn read_bucket(&mut self, bucket: u32) -> Result<MdbPtr> {
        let pos = (MDB_PTR_SIZE as u64) * (bucket as u64 + 1);
        self.fp_index.seek(SeekFrom::Start(pos))?;
        let mut buf = [0u8; MDB_PTR_SIZE];
        self.fp_index.read_exact(&mut buf)?;
        Ok(MdbPtr::from_le_bytes(buf))
    }

    fn read_index(&mut self, idxptr: MdbPtr) -> Result<MdbIndex> {
        self.fp_index.seek(SeekFrom::Start(idxptr as u64))?;
        let mut ptr_buf = [0u8; MDB_PTR_SIZE];
        self.fp_index.read_exact(&mut ptr_buf)?;
        let next_ptr = MdbPtr::from_le_bytes(ptr_buf);

        let key_len = self.options.key_size_max as usize;
        let mut key = vec![0u8; key_len];
        self.fp_index.read_exact(&mut key)?;

        self.fp_index.read_exact(&mut ptr_buf)?;
        let value_ptr = MdbPtr::from_le_bytes(ptr_buf);

        let mut size_buf = [0u8; MDB_DATALEN_SIZE];
        self.fp_index.read_exact(&mut size_buf)?;
        let value_size = MdbSize::from_le_bytes(size_buf);

        Ok(MdbIndex {
            next_ptr,
            value_ptr,
            value_size,
            key,
        })
    }

    fn write_bucket(&mut self, bucket: u32, ptr: MdbPtr) -> Result<()> {
        let pos = (MDB_PTR_SIZE as u64) * (bucket as u64 + 1);
        self.fp_index.seek(SeekFrom::Start(pos))?;
        self.fp_index.write_all(&ptr.to_le_bytes())?;
        self.fp_index.flush()?;
        Ok(())
    }

    fn write_index(
        &mut self,
        idxptr: MdbPtr,
        key: &[u8],
        value_ptr: MdbPtr,
        value_size: MdbSize,
    ) -> Result<()> {
        // Seek past the next_ptr field and write the key bytes (no padding,
        // matching the C implementation which relies on prior zero-fill).
        let key_pos = idxptr as u64 + MDB_PTR_SIZE as u64;
        self.fp_index.seek(SeekFrom::Start(key_pos))?;
        self.fp_index.write_all(key)?;

        // Seek past the key region to write the value pointer and value size.
        let value_ptr_pos = key_pos + self.options.key_size_max as u64;
        self.fp_index.seek(SeekFrom::Start(value_ptr_pos))?;
        self.fp_index.write_all(&value_ptr.to_le_bytes())?;
        self.fp_index.write_all(&value_size.to_le_bytes())?;
        self.fp_index.flush()?;
        Ok(())
    }

    fn read_nextptr(&mut self, idxptr: MdbPtr) -> Result<MdbPtr> {
        self.fp_index.seek(SeekFrom::Start(idxptr as u64))?;
        let mut buf = [0u8; MDB_PTR_SIZE];
        self.fp_index.read_exact(&mut buf)?;
        Ok(MdbPtr::from_le_bytes(buf))
    }

    fn write_nextptr(&mut self, ptr: MdbPtr, nextptr: MdbPtr) -> Result<()> {
        self.fp_index.seek(SeekFrom::Start(ptr as u64))?;
        self.fp_index.write_all(&nextptr.to_le_bytes())?;
        self.fp_index.flush()?;
        Ok(())
    }

    fn read_data(
        &mut self,
        valptr: MdbPtr,
        valsize: MdbSize,
        buf: &mut [u8],
    ) -> Result<usize> {
        let valsize_us = valsize as usize;
        // Mirror C: the supplied buffer must have room for the data plus a NUL
        // terminator byte.
        if buf.len() < valsize_us + 1 {
            return Err(MdbError::BufferSizeTooSmall);
        }
        self.fp_data.seek(SeekFrom::Start(valptr as u64))?;
        self.fp_data.read_exact(&mut buf[..valsize_us])?;
        if buf.len() > valsize_us {
            buf[valsize_us] = 0;
        }
        Ok(valsize_us)
    }

    fn write_data(&mut self, valptr: MdbPtr, value: &[u8], valsize: MdbSize) -> Result<()> {
        let valsize_us = valsize as usize;
        self.fp_data.seek(SeekFrom::Start(valptr as u64))?;
        self.fp_data.write_all(&value[..valsize_us])?;
        self.fp_data.flush()?;
        Ok(())
    }

    fn stretch_index_file(&mut self, ptr: &mut MdbPtr) -> Result<()> {
        let pos = self.fp_index.seek(SeekFrom::End(0))?;
        *ptr = pos as MdbPtr;
        let zeros = vec![0u8; self.index_record_size as usize];
        self.fp_index.write_all(&zeros)?;
        self.fp_index.flush()?;
        Ok(())
    }

    fn index_alloc(&mut self, ptr: &mut MdbPtr) -> Result<()> {
        let freeptr = self.read_nextptr(0)?;
        if freeptr != 0 {
            let new_freeptr = self.read_nextptr(freeptr)?;
            self.write_nextptr(0, new_freeptr)?;
            self.write_nextptr(freeptr, 0)?;
            *ptr = freeptr;
            Ok(())
        } else {
            self.stretch_index_file(ptr)
        }
    }

    fn data_alloc(&mut self, size: MdbSize, ptr: &mut MdbPtr) -> Result<()> {
        // Read the entire data file into memory once and emulate the
        // byte-by-byte scanning used in the C implementation.
        let file_size = self.fp_data.seek(SeekFrom::End(0))?;
        self.fp_data.seek(SeekFrom::Start(0))?;
        let mut content = vec![0u8; file_size as usize];
        self.fp_data.read_exact(&mut content)?;

        let n = content.len();
        let mut i: usize = 0;

        while i < n {
            let mut byte = content[i];
            i += 1;

            let mut at_eof = false;

            // Inner1: skip non-zero bytes until a zero byte or EOF is found.
            while byte != 0 {
                if i >= n {
                    at_eof = true;
                    break;
                }
                byte = content[i];
                i += 1;
            }

            let start_ptr = i as MdbPtr;

            if !at_eof {
                // Inner2: skip zero bytes until a non-zero byte or EOF is found.
                while byte == 0 {
                    if i >= n {
                        at_eof = true;
                        break;
                    }
                    byte = content[i];
                    i += 1;
                }
            }

            let end_ptr = i as MdbPtr;

            if end_ptr.saturating_sub(start_ptr) >= size + 2 {
                *ptr = start_ptr + 1;
                return Ok(());
            }

            if at_eof {
                break;
            }
        }

        // No suitable gap; extend the data file.
        let end_ptr = self.fp_data.seek(SeekFrom::End(0))? as MdbPtr;
        let zeros = vec![0u8; size as usize];
        self.fp_data.write_all(&zeros)?;
        self.fp_data.flush()?;
        *ptr = end_ptr;
        Ok(())
    }

    fn index_free(&mut self, ptr: MdbPtr) -> Result<()> {
        // Read the current freeptr.
        self.fp_index.seek(SeekFrom::Start(0))?;
        let mut buf = [0u8; MDB_PTR_SIZE];
        self.fp_index.read_exact(&mut buf)?;
        let freeptr = MdbPtr::from_le_bytes(buf);

        // Write our ptr as the new freeptr (head of the free list).
        self.fp_index.seek(SeekFrom::Start(0))?;
        self.fp_index.write_all(&ptr.to_le_bytes())?;

        // Set this slot's next pointer to the previous freeptr.
        self.fp_index.seek(SeekFrom::Start(ptr as u64))?;
        self.fp_index.write_all(&freeptr.to_le_bytes())?;

        // Zero out the key region so a future allocation sees clean padding.
        let zeros = vec![0u8; self.options.key_size_max as usize];
        self.fp_index.write_all(&zeros)?;

        self.fp_index.flush()?;
        Ok(())
    }

    fn data_free(&mut self, ptr: MdbPtr, size: MdbSize) -> Result<()> {
        self.fp_data.seek(SeekFrom::Start(ptr as u64))?;
        let zeros = vec![0u8; size as usize];
        self.fp_data.write_all(&zeros)?;
        self.fp_data.flush()?;
        Ok(())
    }

    fn alloc() -> Result<()> {
        Ok(())
    }

    fn free() -> Result<()> {
        Ok(())
    }

    fn hash(&self, key: &str) -> u32 {
        let mut ret: u32 = 0;
        for (i, &c) in key.as_bytes().iter().enumerate() {
            ret = ret.wrapping_add((c as u32).wrapping_mul(i as u32));
        }
        ret
    }

    fn close(&mut self) -> Result<()> {
        let _ = self.fp_index.flush();
        let _ = self.fp_data.flush();
        Ok(())
    }
}

impl Mdb {
    fn key_matches(stored: &[u8], key: &[u8]) -> bool {
        // Stored key is zero-padded to key_size_max bytes; compare up to the
        // first NUL byte, like C's strcmp on the stored buffer.
        let len = stored.iter().position(|&b| b == 0).unwrap_or(stored.len());
        &stored[..len] == key
    }
}

pub fn mdb_status() -> Result<MdbStatus> {
    Ok(MdbStatus {
        code: 0,
        desc: String::new(),
    })
}

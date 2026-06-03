use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
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

fn key_matches(stored: &[u8], target: &str) -> bool {
    let target_bytes = target.as_bytes();
    let null_pos = stored.iter().position(|&b| b == 0).unwrap_or(stored.len());
    &stored[..null_pos] == target_bytes
}

impl Mdb {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();
        let path_str = path_ref.to_string_lossy().to_string();
        let super_path = PathBuf::from(format!("{}.db.super", path_str));
        let index_path = PathBuf::from(format!("{}.db.index", path_str));
        let data_path = PathBuf::from(format!("{}.db.data", path_str));

        let mut fp_superblock = File::open(&super_path)?;

        let mut content = String::new();
        fp_superblock.read_to_string(&mut content)?;
        let mut tokens = content.split_whitespace();

        let parse_err = || io::Error::new(io::ErrorKind::InvalidData, "bad superblock");

        let db_name = tokens.next().ok_or_else(parse_err)?.to_string();
        let key_size_max: u16 = tokens
            .next()
            .ok_or_else(parse_err)?
            .parse()
            .map_err(|_| parse_err())?;
        let data_size_max: u32 = tokens
            .next()
            .ok_or_else(parse_err)?
            .parse()
            .map_err(|_| parse_err())?;
        let hash_buckets: u32 = tokens
            .next()
            .ok_or_else(parse_err)?
            .parse()
            .map_err(|_| parse_err())?;
        let items_max: u32 = tokens
            .next()
            .ok_or_else(parse_err)?
            .parse()
            .map_err(|_| parse_err())?;

        let options = MdbOptions {
            db_name: db_name.clone(),
            key_size_max,
            data_size_max,
            hash_buckets,
            items_max,
        };

        let index_record_size =
            key_size_max as u32 + (MDB_PTR_SIZE as u32) * 2 + MDB_DATALEN_SIZE as u32;

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
        let path_ref = path.as_ref();
        let path_str = path_ref.to_string_lossy().to_string();
        let super_path = PathBuf::from(format!("{}.db.super", path_str));
        let index_path = PathBuf::from(format!("{}.db.index", path_str));
        let data_path = PathBuf::from(format!("{}.db.data", path_str));

        let db_name = options.db_name.clone();
        let index_record_size =
            options.key_size_max as u32 + (MDB_PTR_SIZE as u32) * 2 + MDB_DATALEN_SIZE as u32;

        let mut fp_superblock = OpenOptions::new()
            .read(true)
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

        let mut fp_index = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&index_path)?;

        let zero_ptr: MdbPtr = 0;
        // Write free pointer
        fp_index.write_all(&zero_ptr.to_le_bytes())?;
        // Write hash buckets
        for _ in 0..options.hash_buckets {
            fp_index.write_all(&zero_ptr.to_le_bytes())?;
        }
        fp_index.flush()?;

        let fp_data = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
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
    pub fn read(&mut self, key: &str, buf: &mut [u8]) -> Result<usize> {
        let bucket = self.hash(key) % self.options.hash_buckets;
        let mut ptr = self.read_bucket(bucket)?;

        while ptr != 0 {
            let index = self.read_index(ptr)?;
            if key_matches(&index.key, key) {
                return self.read_data(index.value_ptr, index.value_size, buf);
            }
            ptr = index.next_ptr;
        }

        Err(MdbError::KeyNotFound)
    }
    pub fn write(&mut self, key: &str, value: &str) -> Result<()> {
        let bucket = self.hash(key) % self.options.hash_buckets;

        let key_size = key.len() as u32;
        if key_size > self.options.key_size_max as u32 {
            return Err(MdbError::KeySizeTooLarge);
        }
        let value_size = value.len() as MdbSize;
        if value_size > self.options.data_size_max {
            return Err(MdbError::ValueSizeTooLarge);
        }

        let mut save_ptr: MdbPtr = (MDB_PTR_SIZE as MdbPtr) * (bucket + 1);
        let mut ptr = self.read_bucket(bucket)?;

        let mut found_idx: Option<MdbIndex> = None;
        while ptr != 0 {
            let index = self.read_index(ptr)?;
            if key_matches(&index.key, key) {
                found_idx = Some(index);
                break;
            }
            save_ptr = ptr;
            ptr = index.next_ptr;
        }

        if ptr == 0 {
            // New entry
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

            if let Err(e) = self.write_index(index_ptr, key.as_bytes(), value_ptr, value_size) {
                let _ = self.data_free(value_ptr, value_size);
                let _ = self.index_free(index_ptr);
                return Err(e);
            }

            if let Err(e) = self.write_nextptr(save_ptr, index_ptr) {
                let _ = self.data_free(value_ptr, value_size);
                let _ = self.index_free(index_ptr);
                return Err(e);
            }

            Ok(())
        } else {
            // Update existing entry
            let index = found_idx.unwrap();
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

        let mut save_ptr: MdbPtr = (MDB_PTR_SIZE as MdbPtr) * (bucket + 1);
        let mut ptr = self.read_bucket(bucket)?;

        let mut found_idx: Option<MdbIndex> = None;
        while ptr != 0 {
            let index = self.read_index(ptr)?;
            if key_matches(&index.key, key) {
                found_idx = Some(index);
                break;
            }
            save_ptr = ptr;
            ptr = index.next_ptr;
        }

        if ptr == 0 {
            return Err(MdbError::KeyNotFound);
        }

        let index = found_idx.unwrap();
        self.data_free(index.value_ptr, index.value_size)?;
        self.index_free(ptr)?;
        self.write_nextptr(save_ptr, index.next_ptr)?;

        Ok(())
    }
    pub fn get_options(&self) -> &MdbOptions {
        &self.options
    }
    pub fn index_size(&mut self) -> Result<u64> {
        let size = self.fp_index.seek(SeekFrom::End(0))?;
        Ok(size)
    }
    pub fn data_size(&mut self) -> Result<u64> {
        let size = self.fp_data.seek(SeekFrom::End(0))?;
        Ok(size)
    }
    // Private helper methods
    fn read_bucket(&mut self, bucket: u32) -> Result<MdbPtr> {
        let pos = (MDB_PTR_SIZE as u64) * ((bucket as u64) + 1);
        self.fp_index.seek(SeekFrom::Start(pos))?;
        let mut buf = [0u8; 4];
        self.fp_index.read_exact(&mut buf)?;
        Ok(MdbPtr::from_le_bytes(buf))
    }
    fn read_index(&mut self, idxptr: MdbPtr) -> Result<MdbIndex> {
        self.fp_index.seek(SeekFrom::Start(idxptr as u64))?;

        let mut next_ptr_buf = [0u8; 4];
        self.fp_index.read_exact(&mut next_ptr_buf)?;
        let next_ptr = MdbPtr::from_le_bytes(next_ptr_buf);

        let mut key = vec![0u8; self.options.key_size_max as usize];
        self.fp_index.read_exact(&mut key)?;

        let mut value_ptr_buf = [0u8; 4];
        self.fp_index.read_exact(&mut value_ptr_buf)?;
        let value_ptr = MdbPtr::from_le_bytes(value_ptr_buf);

        let mut value_size_buf = [0u8; 4];
        self.fp_index.read_exact(&mut value_size_buf)?;
        let value_size = MdbSize::from_le_bytes(value_size_buf);

        Ok(MdbIndex {
            next_ptr,
            value_ptr,
            value_size,
            key,
        })
    }
    fn write_bucket(&mut self, bucket: u32, ptr: MdbPtr) -> Result<()> {
        let pos = (MDB_PTR_SIZE as u64) * ((bucket as u64) + 1);
        self.fp_index.seek(SeekFrom::Start(pos))?;
        self.fp_index.write_all(&ptr.to_le_bytes())?;
        self.fp_index.flush()?;
        Ok(())
    }
    fn write_index(&mut self, idxptr: MdbPtr, key: &[u8], value_ptr: MdbPtr, value_size: MdbSize) -> Result<()> {
        // Skip next_ptr field
        self.fp_index.seek(SeekFrom::Start((idxptr as u64) + MDB_PTR_SIZE as u64))?;
        // Write the key (only its actual length, not key_size_max)
        self.fp_index.write_all(key)?;

        // Seek to value_ptr/value_size field
        let value_ptr_pos =
            (idxptr as u64) + MDB_PTR_SIZE as u64 + self.options.key_size_max as u64;
        self.fp_index.seek(SeekFrom::Start(value_ptr_pos))?;
        self.fp_index.write_all(&value_ptr.to_le_bytes())?;
        self.fp_index.write_all(&value_size.to_le_bytes())?;
        self.fp_index.flush()?;
        Ok(())
    }
    fn read_nextptr(&mut self, idxptr: MdbPtr) -> Result<MdbPtr> {
        self.fp_index.seek(SeekFrom::Start(idxptr as u64))?;
        let mut buf = [0u8; 4];
        self.fp_index.read_exact(&mut buf)?;
        Ok(MdbPtr::from_le_bytes(buf))
    }
    fn write_nextptr(&mut self, ptr: MdbPtr, nextptr: MdbPtr) -> Result<()> {
        self.fp_index.seek(SeekFrom::Start(ptr as u64))?;
        self.fp_index.write_all(&nextptr.to_le_bytes())?;
        self.fp_index.flush()?;
        Ok(())
    }
    fn read_data(&mut self, valptr: MdbPtr, valsize: MdbSize, buf: &mut [u8]) -> Result<usize> {
        if buf.len() < (valsize as usize) + 1 {
            return Err(MdbError::BufferSizeTooSmall);
        }
        self.fp_data.seek(SeekFrom::Start(valptr as u64))?;
        self.fp_data.read_exact(&mut buf[..valsize as usize])?;
        buf[valsize as usize] = 0;
        Ok(valsize as usize)
    }
    fn write_data(&mut self, valptr: MdbPtr, value: &[u8], valsize: MdbSize) -> Result<()> {
        self.fp_data.seek(SeekFrom::Start(valptr as u64))?;
        self.fp_data.write_all(&value[..valsize as usize])?;
        self.fp_data.flush()?;
        Ok(())
    }
    fn stretch_index_file(&mut self, ptr: &mut MdbPtr) -> Result<()> {
        let end = self.fp_index.seek(SeekFrom::End(0))?;
        *ptr = end as MdbPtr;
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
        self.fp_data.seek(SeekFrom::Start(0))?;

        let mut byte: u8 = 0;
        let mut eof = false;
        let mut buf = [0u8; 1];

        while !eof {
            // Read one byte
            let n = self.fp_data.read(&mut buf)?;
            if n == 0 {
                eof = true;
            } else {
                byte = buf[0];
            }

            // Skip non-zero bytes
            while !eof && byte != 0 {
                let n = self.fp_data.read(&mut buf)?;
                if n == 0 {
                    eof = true;
                } else {
                    byte = buf[0];
                }
            }

            let start_ptr = self.fp_data.stream_position()? as MdbPtr;

            // Skip zero bytes
            while !eof && byte == 0 {
                let n = self.fp_data.read(&mut buf)?;
                if n == 0 {
                    eof = true;
                } else {
                    byte = buf[0];
                }
            }

            let end_ptr = self.fp_data.stream_position()? as MdbPtr;

            // Note: we rely on the extra padding (+2) to keep correct results
            if end_ptr.wrapping_sub(start_ptr) >= size + 2 {
                *ptr = start_ptr + 1;
                return Ok(());
            }
        }

        // Append at end
        let end_ptr = self.fp_data.stream_position()? as MdbPtr;
        let zeros = vec![0u8; size as usize];
        self.fp_data.write_all(&zeros)?;
        self.fp_data.flush()?;
        *ptr = end_ptr;
        Ok(())
    }
    fn index_free(&mut self, ptr: MdbPtr) -> Result<()> {
        // Read current free pointer (at position 0)
        self.fp_index.seek(SeekFrom::Start(0))?;
        let mut buf = [0u8; 4];
        self.fp_index.read_exact(&mut buf)?;
        let freeptr = MdbPtr::from_le_bytes(buf);

        // Update free pointer to point to this index
        self.fp_index.seek(SeekFrom::Start(0))?;
        self.fp_index.write_all(&ptr.to_le_bytes())?;

        // At this index position, write the previous free pointer (chains free list)
        self.fp_index.seek(SeekFrom::Start(ptr as u64))?;
        self.fp_index.write_all(&freeptr.to_le_bytes())?;

        // Zero out the key portion
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
    fn alloc()-> Result<()> {
        Ok(())
    }
    fn free() -> Result<()> {
        Ok(())
    }
    fn hash(&self, key: &str) -> u32 {
        let mut ret: u32 = 0;
        for (i, &b) in key.as_bytes().iter().enumerate() {
            ret = ret.wrapping_add((b as u32).wrapping_mul(i as u32));
        }
        ret
    }
    fn close(&mut self) -> Result<()> {
        let _ = self.fp_superblock.flush();
        let _ = self.fp_index.flush();
        let _ = self.fp_data.flush();
        Ok(())
    }
} // impl Mdb
pub fn mdb_status() -> Result<MdbStatus> {
    Ok(MdbStatus {
        code: 0,
        desc: String::new(),
    })
}

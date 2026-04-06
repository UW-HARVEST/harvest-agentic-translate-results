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
impl Mdb {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();

        let mut fp_superblock = File::open(format!("{}.super", path_str))
            .map_err(|e| MdbError::Io(e))?;

        let mut content = String::new();
        fp_superblock.read_to_string(&mut content)?;
        let mut parts = content.split_whitespace();

        let db_name = parts.next().unwrap_or("").to_string();
        let key_size_max: u16 = parts.next().unwrap_or("0").parse().unwrap_or(0);
        let data_size_max: u32 = parts.next().unwrap_or("0").parse().unwrap_or(0);
        let hash_buckets: u32 = parts.next().unwrap_or("0").parse().unwrap_or(0);
        let items_max: u32 = parts.next().unwrap_or("0").parse().unwrap_or(0);

        let index_record_size = key_size_max as u32 + MDB_PTR_SIZE as u32 * 2 + MDB_DATALEN_SIZE as u32;

        let fp_index = OpenOptions::new().read(true).write(true)
            .open(format!("{}.index", path_str))?;
        let fp_data = OpenOptions::new().read(true).write(true)
            .open(format!("{}.data", path_str))?;

        Ok(Mdb {
            db_name: db_name.clone(),
            fp_superblock,
            fp_index,
            fp_data,
            options: MdbOptions { db_name, key_size_max, data_size_max, hash_buckets, items_max },
            index_record_size,
        })
    }

    pub fn create<P: AsRef<Path>>(path: P, options: MdbOptions) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();

        let index_record_size = options.key_size_max as u32 + MDB_PTR_SIZE as u32 * 2 + MDB_DATALEN_SIZE as u32;

        let mut fp_superblock = File::create(format!("{}.super", path_str))?;
        write!(fp_superblock, "{}\n{}\n{}\n{}\n{}\n",
            options.db_name, options.key_size_max, options.data_size_max,
            options.hash_buckets, options.items_max)?;
        fp_superblock.flush()?;

        let mut fp_index = OpenOptions::new().read(true).write(true).create(true).truncate(true)
            .open(format!("{}.index", path_str))?;

        // Write freeptr (0) + hash_buckets zero pointers
        let zero_ptr: MdbPtr = 0;
        let zero_bytes = zero_ptr.to_le_bytes();
        for _ in 0..=options.hash_buckets {
            fp_index.write_all(&zero_bytes)?;
        }
        fp_index.flush()?;

        let fp_data = OpenOptions::new().read(true).write(true).create(true).truncate(true)
            .open(format!("{}.data", path_str))?;

        Ok(Mdb {
            db_name: options.db_name.clone(),
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
            let key_str = std::str::from_utf8(&index.key).unwrap_or("");
            let key_trimmed = key_str.trim_end_matches('\0');
            if key_trimmed == key {
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
        let value_size = value.len() as MdbSize;
        if value_size > self.options.data_size_max {
            return Err(MdbError::ValueSizeTooLarge);
        }

        let mut save_ptr = MDB_PTR_SIZE as MdbPtr * (bucket + 1);
        let mut ptr = self.read_bucket(bucket)?;

        while ptr != 0 {
            let index = self.read_index(ptr)?;
            let key_str = std::str::from_utf8(&index.key).unwrap_or("");
            let key_trimmed = key_str.trim_end_matches('\0');
            if key_trimmed == key {
                break;
            }
            save_ptr = ptr;
            ptr = index.next_ptr;
        }

        if ptr == 0 {
            // New key
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
            // Existing key - update
            let index = self.read_index(ptr)?;
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
        let mut save_ptr = MDB_PTR_SIZE as MdbPtr * (bucket + 1);
        let mut ptr = self.read_bucket(bucket)?;

        while ptr != 0 {
            let index = self.read_index(ptr)?;
            let key_str = std::str::from_utf8(&index.key).unwrap_or("");
            let key_trimmed = key_str.trim_end_matches('\0');
            if key_trimmed == key {
                // Found it
                self.data_free(index.value_ptr, index.value_size)?;
                self.index_free(ptr)?;
                self.write_nextptr(save_ptr, index.next_ptr)?;
                return Ok(());
            }
            save_ptr = ptr;
            ptr = index.next_ptr;
        }

        Err(MdbError::KeyNotFound)
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
        self.fp_index.seek(SeekFrom::Start((MDB_PTR_SIZE as u64) * (bucket as u64 + 1)))?;
        let mut buf = [0u8; MDB_PTR_SIZE];
        self.fp_index.read_exact(&mut buf)?;
        Ok(MdbPtr::from_le_bytes(buf))
    }

    fn read_index(&mut self, idxptr: MdbPtr) -> Result<MdbIndex> {
        self.fp_index.seek(SeekFrom::Start(idxptr as u64))?;

        let mut buf4 = [0u8; MDB_PTR_SIZE];
        self.fp_index.read_exact(&mut buf4)?;
        let next_ptr = MdbPtr::from_le_bytes(buf4);

        let mut key = vec![0u8; self.options.key_size_max as usize];
        self.fp_index.read_exact(&mut key)?;

        self.fp_index.read_exact(&mut buf4)?;
        let value_ptr = MdbPtr::from_le_bytes(buf4);

        let mut buf_sz = [0u8; MDB_DATALEN_SIZE];
        self.fp_index.read_exact(&mut buf_sz)?;
        let value_size = MdbSize::from_le_bytes(buf_sz);

        Ok(MdbIndex { next_ptr, value_ptr, value_size, key })
    }

    fn write_bucket(&mut self, bucket: u32, ptr: MdbPtr) -> Result<()> {
        self.fp_index.seek(SeekFrom::Start((MDB_PTR_SIZE as u64) * (bucket as u64 + 1)))?;
        self.fp_index.write_all(&ptr.to_le_bytes())?;
        self.fp_index.flush()?;
        Ok(())
    }

    fn write_index(&mut self, idxptr: MdbPtr, key: &[u8], value_ptr: MdbPtr, value_size: MdbSize) -> Result<()> {
        // Seek past next_ptr to key position
        self.fp_index.seek(SeekFrom::Start(idxptr as u64 + MDB_PTR_SIZE as u64))?;
        self.fp_index.write_all(key)?;

        // Seek to value_ptr position (after key_size_max bytes)
        let value_ptr_pos = idxptr as u64 + MDB_PTR_SIZE as u64 + self.options.key_size_max as u64;
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

    fn read_data(&mut self, valptr: MdbPtr, valsize: MdbSize, buf: &mut [u8]) -> Result<usize> {
        if buf.len() < valsize as usize + 1 {
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
        *ptr = self.fp_index.seek(SeekFrom::End(0))? as MdbPtr;
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

        // Read entire data file into memory for scanning
        let mut data = Vec::new();
        self.fp_data.read_to_end(&mut data)?;

        let len = data.len();
        let mut pos = 0usize;

        while pos < len {
            // Skip non-zero bytes
            while pos < len && data[pos] != 0 {
                pos += 1;
            }

            let start = pos;

            // Count zero bytes
            while pos < len && data[pos] == 0 {
                pos += 1;
            }

            let zero_run = pos - start;
            if zero_run >= size as usize + 2 {
                *ptr = (start + 1) as MdbPtr;
                return Ok(());
            }
        }

        // No suitable gap found, extend file
        let end = len as MdbPtr;
        self.fp_data.seek(SeekFrom::End(0))?;
        let zeros = vec![0u8; size as usize];
        self.fp_data.write_all(&zeros)?;
        self.fp_data.flush()?;
        *ptr = end;
        Ok(())
    }

    fn index_free(&mut self, ptr: MdbPtr) -> Result<()> {
        // Read current freeptr
        self.fp_index.seek(SeekFrom::Start(0))?;
        let mut buf = [0u8; MDB_PTR_SIZE];
        self.fp_index.read_exact(&mut buf)?;
        let freeptr = MdbPtr::from_le_bytes(buf);

        // Write ptr as new freeptr
        self.fp_index.seek(SeekFrom::Start(0))?;
        self.fp_index.write_all(&ptr.to_le_bytes())?;

        // At ptr, write old freeptr as next + zero out key
        self.fp_index.seek(SeekFrom::Start(ptr as u64))?;
        self.fp_index.write_all(&freeptr.to_le_bytes())?;
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
        for (i, byte) in key.bytes().enumerate() {
            ret = ret.wrapping_add((byte as u32).wrapping_mul(i as u32));
        }
        ret
    }

    fn close(&mut self) -> Result<()> {
        Ok(())
    }
} // impl Mdb
pub fn mdb_status() -> Result<MdbStatus> {
    Ok(MdbStatus { code: 0, desc: String::new() })
}

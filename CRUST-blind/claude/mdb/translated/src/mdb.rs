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

fn build_path<P: AsRef<Path>>(base: P, suffix: &str) -> PathBuf {
    let mut s = base.as_ref().as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

impl Mdb {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let super_path = build_path(&path, ".db.super");
        let index_path = build_path(&path, ".db.index");
        let data_path = build_path(&path, ".db.data");

        let mut fp_super = File::open(&super_path)?;
        let mut content = String::new();
        fp_super.read_to_string(&mut content)?;

        let mut tokens = content.split_whitespace();
        let invalid_data = || io::Error::new(io::ErrorKind::InvalidData, "invalid superblock");
        let db_name = tokens
            .next()
            .ok_or_else(invalid_data)?
            .to_string();
        // Limit db_name to DB_NAME_MAX as a safety measure (mirrors C's allocation).
        let db_name = if db_name.len() > DB_NAME_MAX {
            db_name[..DB_NAME_MAX].to_string()
        } else {
            db_name
        };
        let key_size_max: u16 = tokens
            .next()
            .ok_or_else(invalid_data)?
            .parse()
            .map_err(|_| MdbError::Io(invalid_data()))?;
        let data_size_max: u32 = tokens
            .next()
            .ok_or_else(invalid_data)?
            .parse()
            .map_err(|_| MdbError::Io(invalid_data()))?;
        let hash_buckets: u32 = tokens
            .next()
            .ok_or_else(invalid_data)?
            .parse()
            .map_err(|_| MdbError::Io(invalid_data()))?;
        let items_max: u32 = tokens
            .next()
            .ok_or_else(invalid_data)?
            .parse()
            .map_err(|_| MdbError::Io(invalid_data()))?;

        let options = MdbOptions {
            db_name: db_name.clone(),
            key_size_max,
            data_size_max,
            hash_buckets,
            items_max,
        };

        let index_record_size = (key_size_max as u32)
            + (MDB_PTR_SIZE as u32) * 2
            + (MDB_DATALEN_SIZE as u32);

        let fp_index = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&index_path)?;
        let fp_data = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&data_path)?;

        Ok(Self {
            db_name,
            fp_superblock: fp_super,
            fp_index,
            fp_data,
            options,
            index_record_size,
        })
    }

    pub fn create<P: AsRef<Path>>(path: P, options: MdbOptions) -> Result<Self> {
        let super_path = build_path(&path, ".db.super");
        let index_path = build_path(&path, ".db.index");
        let data_path = build_path(&path, ".db.data");

        let mut fp_super = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&super_path)?;

        // Write superblock metadata as whitespace-separated tokens.
        write!(
            fp_super,
            "{}\n{}\n{}\n{}\n{}\n",
            options.db_name,
            options.key_size_max,
            options.data_size_max,
            options.hash_buckets,
            options.items_max
        )?;
        fp_super.flush()?;

        let mut fp_index = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&index_path)?;

        // Write zero free pointer + zero hash buckets.
        let zero_ptr: MdbPtr = 0;
        fp_index.write_all(&zero_ptr.to_le_bytes())?;
        let zero_bucket = vec![0u8; MDB_PTR_SIZE];
        for _ in 0..options.hash_buckets {
            fp_index.write_all(&zero_bucket)?;
        }
        fp_index.flush()?;

        let fp_data = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&data_path)?;

        let index_record_size = (options.key_size_max as u32)
            + (MDB_PTR_SIZE as u32) * 2
            + (MDB_DATALEN_SIZE as u32);
        let db_name = options.db_name.clone();

        Ok(Self {
            db_name,
            fp_superblock: fp_super,
            fp_index,
            fp_data,
            options,
            index_record_size,
        })
    }

    pub fn read(&mut self, key: &str, buf: &mut [u8]) -> Result<usize> {
        if self.options.hash_buckets == 0 {
            return Err(MdbError::KeyNotFound);
        }
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
        if key.len() > self.options.key_size_max as usize {
            return Err(MdbError::KeySizeTooLarge);
        }
        if value.len() > self.options.data_size_max as usize {
            return Err(MdbError::ValueSizeTooLarge);
        }
        if self.options.hash_buckets == 0 {
            // Without hash buckets there is nowhere to chain entries.
            return Err(MdbError::AllocationFailed);
        }

        let bucket = self.hash(key) % self.options.hash_buckets;
        let value_size = value.len() as MdbSize;

        let mut save_ptr: MdbPtr =
            (MDB_PTR_SIZE as MdbPtr).wrapping_mul(bucket.wrapping_add(1));
        let mut ptr = self.read_bucket(bucket)?;
        let mut existing: Option<MdbIndex> = None;

        while ptr != 0 {
            let index = self.read_index(ptr)?;
            if Self::key_matches(&index.key, key.as_bytes()) {
                existing = Some(index);
                break;
            }
            save_ptr = ptr;
            ptr = index.next_ptr;
        }

        if ptr == 0 {
            // New entry path.
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

            if let Err(e) = self.write_nextptr(save_ptr, index_ptr) {
                let _ = self.data_free(value_ptr, value_size);
                let _ = self.index_free(index_ptr);
                return Err(e);
            }
            Ok(())
        } else {
            // Update path: free old data, allocate new, rewrite index.
            let existing = existing.expect("existing index must be set");
            self.data_free(existing.value_ptr, existing.value_size)?;

            let mut value_ptr: MdbPtr = 0;
            self.data_alloc(value_size, &mut value_ptr)?;
            self.write_data(value_ptr, value.as_bytes(), value_size)?;
            self.write_index(ptr, key.as_bytes(), value_ptr, value_size)?;
            Ok(())
        }
    }

    pub fn delete(&mut self, key: &str) -> Result<()> {
        if self.options.hash_buckets == 0 {
            return Err(MdbError::KeyNotFound);
        }
        let bucket = self.hash(key) % self.options.hash_buckets;
        let mut save_ptr: MdbPtr =
            (MDB_PTR_SIZE as MdbPtr).wrapping_mul(bucket.wrapping_add(1));
        let mut ptr = self.read_bucket(bucket)?;
        let mut found: Option<(MdbPtr, MdbIndex)> = None;

        while ptr != 0 {
            let index = self.read_index(ptr)?;
            if Self::key_matches(&index.key, key.as_bytes()) {
                found = Some((ptr, index));
                break;
            }
            save_ptr = ptr;
            ptr = index.next_ptr;
        }

        let (found_ptr, index) = match found {
            Some(x) => x,
            None => return Err(MdbError::KeyNotFound),
        };

        self.data_free(index.value_ptr, index.value_size)?;
        self.index_free(found_ptr)?;
        self.write_nextptr(save_ptr, index.next_ptr)?;

        Ok(())
    }

    pub fn get_options(&self) -> &MdbOptions {
        &self.options
    }

    pub fn index_size(&mut self) -> Result<u64> {
        let pos = self.fp_index.seek(SeekFrom::End(0))?;
        Ok(pos)
    }

    pub fn data_size(&mut self) -> Result<u64> {
        let pos = self.fp_data.seek(SeekFrom::End(0))?;
        Ok(pos)
    }

    // Private helper methods
    fn read_bucket(&mut self, bucket: u32) -> Result<MdbPtr> {
        let offset = (MDB_PTR_SIZE as u64) * ((bucket as u64) + 1);
        self.fp_index.seek(SeekFrom::Start(offset))?;
        let mut buf = [0u8; MDB_PTR_SIZE];
        self.fp_index.read_exact(&mut buf)?;
        Ok(MdbPtr::from_le_bytes(buf))
    }

    fn read_index(&mut self, idxptr: MdbPtr) -> Result<MdbIndex> {
        self.fp_index.seek(SeekFrom::Start(idxptr as u64))?;

        let mut next_buf = [0u8; MDB_PTR_SIZE];
        self.fp_index.read_exact(&mut next_buf)?;
        let next_ptr = MdbPtr::from_le_bytes(next_buf);

        let key_max = self.options.key_size_max as usize;
        let mut key = vec![0u8; key_max + 1];
        self.fp_index.read_exact(&mut key[..key_max])?;
        // The trailing slot key[key_max] stays 0 to act as a null terminator.

        let mut vp_buf = [0u8; MDB_PTR_SIZE];
        self.fp_index.read_exact(&mut vp_buf)?;
        let value_ptr = MdbPtr::from_le_bytes(vp_buf);

        let mut vs_buf = [0u8; MDB_DATALEN_SIZE];
        self.fp_index.read_exact(&mut vs_buf)?;
        let value_size = MdbSize::from_le_bytes(vs_buf);

        Ok(MdbIndex {
            next_ptr,
            value_ptr,
            value_size,
            key,
        })
    }

    fn write_bucket(&mut self, bucket: u32, ptr: MdbPtr) -> Result<()> {
        let offset = (MDB_PTR_SIZE as u64) * ((bucket as u64) + 1);
        self.fp_index.seek(SeekFrom::Start(offset))?;
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
        // Skip past next_ptr.
        let key_offset = (idxptr as u64) + (MDB_PTR_SIZE as u64);
        self.fp_index.seek(SeekFrom::Start(key_offset))?;
        // Mirror C: write strlen(key) bytes for the key, leaving any extra
        // bytes already present (which will be zero from index_alloc).
        let key_max = self.options.key_size_max as usize;
        let key_len = key.len().min(key_max);
        self.fp_index.write_all(&key[..key_len])?;

        let value_ptr_pos = key_offset + key_max as u64;
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
        if (buf.len() as u64) < (valsize as u64) + 1 {
            return Err(MdbError::BufferSizeTooSmall);
        }
        self.fp_data.seek(SeekFrom::Start(valptr as u64))?;
        self.fp_data.read_exact(&mut buf[..valsize as usize])?;
        if (valsize as usize) < buf.len() {
            buf[valsize as usize] = 0;
        }
        Ok(valsize as usize)
    }

    fn write_data(
        &mut self,
        valptr: MdbPtr,
        value: &[u8],
        valsize: MdbSize,
    ) -> Result<()> {
        let n = valsize as usize;
        if value.len() < n {
            return Err(MdbError::Io(io::Error::new(
                io::ErrorKind::InvalidInput,
                "value buffer too small for declared size",
            )));
        }
        self.fp_data.seek(SeekFrom::Start(valptr as u64))?;
        self.fp_data.write_all(&value[..n])?;
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
        self.fp_data.seek(SeekFrom::Start(0))?;
        let mut byte = [0u8; 1];
        let mut at_eof = false;

        loop {
            // Initial read of one byte for this iteration.
            let n = self.fp_data.read(&mut byte)?;
            if n == 0 {
                at_eof = true;
            }

            // Skip non-zero bytes.
            while !at_eof && byte[0] != 0 {
                let n = self.fp_data.read(&mut byte)?;
                if n == 0 {
                    at_eof = true;
                }
            }

            let start_ptr = self.fp_data.stream_position()? as MdbPtr;

            // Skip zero bytes.
            while !at_eof && byte[0] == 0 {
                let n = self.fp_data.read(&mut byte)?;
                if n == 0 {
                    at_eof = true;
                }
            }

            let end_ptr = self.fp_data.stream_position()? as MdbPtr;

            let span = end_ptr.wrapping_sub(start_ptr);
            let needed = size.wrapping_add(2);
            if span >= needed {
                *ptr = start_ptr.wrapping_add(1);
                return Ok(());
            }

            if at_eof {
                break;
            }
        }

        // No suitable free run found; extend the file.
        let end_ptr = self.fp_data.stream_position()? as MdbPtr;
        let zeros = vec![0u8; size as usize];
        self.fp_data.write_all(&zeros)?;
        self.fp_data.flush()?;
        *ptr = end_ptr;
        Ok(())
    }

    fn index_free(&mut self, ptr: MdbPtr) -> Result<()> {
        // Read current freeptr from start of index file.
        self.fp_index.seek(SeekFrom::Start(0))?;
        let mut freeptr_buf = [0u8; MDB_PTR_SIZE];
        self.fp_index.read_exact(&mut freeptr_buf)?;
        let freeptr = MdbPtr::from_le_bytes(freeptr_buf);

        // Set freeptr to the freed slot.
        self.fp_index.seek(SeekFrom::Start(0))?;
        self.fp_index.write_all(&ptr.to_le_bytes())?;

        // The freed slot's next_ptr links to old freeptr.
        self.fp_index.seek(SeekFrom::Start(ptr as u64))?;
        self.fp_index.write_all(&freeptr.to_le_bytes())?;

        // Zero out the key region of the freed slot.
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
        // Memory allocation happens automatically via Rust's RAII; nothing to do.
        Ok(())
    }

    fn free() -> Result<()> {
        // Memory is reclaimed via Drop; nothing explicit to do.
        Ok(())
    }

    fn hash(&self, key: &str) -> u32 {
        let mut ret: u32 = 0;
        for (i, b) in key.as_bytes().iter().enumerate() {
            // Match the C semantics where `char` is signed on common platforms:
            // (uint32_t)*key sign-extends the byte before promotion.
            let c = ((*b as i8) as i32) as u32;
            ret = ret.wrapping_add(c.wrapping_mul(i as u32));
        }
        ret
    }

    fn close(&mut self) -> Result<()> {
        self.fp_superblock.flush()?;
        self.fp_index.flush()?;
        self.fp_data.flush()?;
        Ok(())
    }

    fn key_matches(stored_key: &[u8], target: &[u8]) -> bool {
        // The stored key is null-terminated; compare up to the first 0 byte.
        let end = stored_key.iter().position(|&b| b == 0).unwrap_or(stored_key.len());
        &stored_key[..end] == target
    }
}
// impl Mdb
pub fn mdb_status() -> Result<MdbStatus> {
    Ok(MdbStatus {
        code: MdbStatusCode::MDB_OK as u8,
        desc: String::new(),
    })
}

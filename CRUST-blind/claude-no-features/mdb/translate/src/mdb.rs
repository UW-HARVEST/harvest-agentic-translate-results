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

fn make_path<P: AsRef<Path>>(path: P, suffix: &str) -> PathBuf {
    let mut s = path.as_ref().as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

impl Mdb {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let super_path = make_path(&path, ".db.super");
        let index_path = make_path(&path, ".db.index");
        let data_path = make_path(&path, ".db.data");

        let mut fp_superblock = File::open(&super_path)?;

        // Read superblock as text
        let mut content = String::new();
        fp_superblock.read_to_string(&mut content)?;
        let mut tokens = content.split_whitespace();

        let db_name = tokens
            .next()
            .ok_or_else(|| MdbError::Io(io::Error::new(io::ErrorKind::InvalidData, "missing db_name")))?
            .to_string();
        let key_size_max: u16 = tokens
            .next()
            .ok_or_else(|| MdbError::Io(io::Error::new(io::ErrorKind::InvalidData, "missing key_size_max")))?
            .parse()
            .map_err(|_| MdbError::Io(io::Error::new(io::ErrorKind::InvalidData, "invalid key_size_max")))?;
        let data_size_max: u32 = tokens
            .next()
            .ok_or_else(|| MdbError::Io(io::Error::new(io::ErrorKind::InvalidData, "missing data_size_max")))?
            .parse()
            .map_err(|_| MdbError::Io(io::Error::new(io::ErrorKind::InvalidData, "invalid data_size_max")))?;
        let hash_buckets: u32 = tokens
            .next()
            .ok_or_else(|| MdbError::Io(io::Error::new(io::ErrorKind::InvalidData, "missing hash_buckets")))?
            .parse()
            .map_err(|_| MdbError::Io(io::Error::new(io::ErrorKind::InvalidData, "invalid hash_buckets")))?;
        let items_max: u32 = tokens
            .next()
            .ok_or_else(|| MdbError::Io(io::Error::new(io::ErrorKind::InvalidData, "missing items_max")))?
            .parse()
            .map_err(|_| MdbError::Io(io::Error::new(io::ErrorKind::InvalidData, "invalid items_max")))?;

        let options = MdbOptions {
            db_name: db_name.clone(),
            key_size_max,
            data_size_max,
            hash_buckets,
            items_max,
        };

        let index_record_size = key_size_max as u32
            + (MDB_PTR_SIZE * 2) as u32
            + MDB_DATALEN_SIZE as u32;

        let fp_index = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&index_path)?;
        let fp_data = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&data_path)?;

        // Validate against DB_NAME_MAX
        if db_name.len() > DB_NAME_MAX {
            return Err(MdbError::Io(io::Error::new(
                io::ErrorKind::InvalidData,
                "db_name too long",
            )));
        }

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
        let super_path = make_path(&path, ".db.super");
        let index_path = make_path(&path, ".db.index");
        let data_path = make_path(&path, ".db.data");

        let db_name = options.db_name.clone();

        let index_record_size = options.key_size_max as u32
            + (MDB_PTR_SIZE * 2) as u32
            + MDB_DATALEN_SIZE as u32;

        // Create superblock
        let mut fp_superblock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&super_path)?;

        writeln!(fp_superblock, "{}", db_name)?;
        writeln!(fp_superblock, "{}", options.key_size_max)?;
        writeln!(fp_superblock, "{}", options.data_size_max)?;
        writeln!(fp_superblock, "{}", options.hash_buckets)?;
        writeln!(fp_superblock, "{}", options.items_max)?;
        fp_superblock.flush()?;

        // Create index file with free_ptr=0 and `hash_buckets` zero pointers.
        let mut fp_index = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&index_path)?;

        let zero_ptr_bytes = (0u32).to_le_bytes();
        // free_ptr at offset 0
        fp_index.write_all(&zero_ptr_bytes)?;
        for _ in 0..options.hash_buckets {
            fp_index.write_all(&zero_ptr_bytes)?;
        }
        fp_index.flush()?;

        // Create data file (empty)
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
            // Trim key at first null byte
            let null_pos = index
                .key
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(index.key.len());
            let stored_key = &index.key[..null_pos];
            if stored_key == key.as_bytes() {
                return self.read_data(index.value_ptr, index.value_size, buf);
            }
            ptr = index.next_ptr;
        }

        Err(MdbError::KeyNotFound)
    }

    pub fn write(&mut self, key: &str, value: &str) -> Result<()> {
        let bucket = self.hash(key) % self.options.hash_buckets;
        let key_bytes = key.as_bytes();
        if key_bytes.len() > self.options.key_size_max as usize {
            return Err(MdbError::KeySizeTooLarge);
        }
        let value_bytes = value.as_bytes();
        if value_bytes.len() > self.options.data_size_max as usize {
            return Err(MdbError::ValueSizeTooLarge);
        }
        let value_size = value_bytes.len() as MdbSize;

        let mut save_ptr: MdbPtr = (MDB_PTR_SIZE as MdbPtr) * (bucket + 1);
        let mut ptr = self.read_bucket(bucket)?;

        // Walk the chain searching for an existing key.
        let mut found_index: Option<MdbIndex> = None;
        while ptr != 0 {
            let index = self.read_index(ptr)?;
            let null_pos = index
                .key
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(index.key.len());
            let stored_key = &index.key[..null_pos];
            if stored_key == key_bytes {
                found_index = Some(index);
                break;
            }
            save_ptr = ptr;
            ptr = index.next_ptr;
        }

        if ptr == 0 {
            // Insert a new entry at end of chain.
            let mut index_ptr: MdbPtr = 0;
            self.index_alloc(&mut index_ptr)?;
            let mut value_ptr: MdbPtr = 0;
            if let Err(e) = self.data_alloc(value_size, &mut value_ptr) {
                let _ = self.index_free(index_ptr);
                return Err(e);
            }
            if let Err(e) = self.write_data(value_ptr, value_bytes, value_size) {
                let _ = self.data_free(value_ptr, value_size);
                let _ = self.index_free(index_ptr);
                return Err(e);
            }
            if let Err(e) = self.write_index(index_ptr, key_bytes, value_ptr, value_size) {
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
            // Update existing entry.
            let existing = found_index.unwrap();
            self.data_free(existing.value_ptr, existing.value_size)?;
            let mut value_ptr: MdbPtr = 0;
            self.data_alloc(value_size, &mut value_ptr)?;
            self.write_data(value_ptr, value_bytes, value_size)?;
            self.write_index(ptr, key_bytes, value_ptr, value_size)?;
            Ok(())
        }
    }

    pub fn delete(&mut self, key: &str) -> Result<()> {
        let bucket = self.hash(key) % self.options.hash_buckets;
        let key_bytes = key.as_bytes();

        let mut save_ptr: MdbPtr = (MDB_PTR_SIZE as MdbPtr) * (bucket + 1);
        let mut ptr = self.read_bucket(bucket)?;

        let mut found_index: Option<MdbIndex> = None;
        while ptr != 0 {
            let index = self.read_index(ptr)?;
            let null_pos = index
                .key
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(index.key.len());
            let stored_key = &index.key[..null_pos];
            if stored_key == key_bytes {
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
        let pos = self.fp_index.seek(SeekFrom::End(0))?;
        Ok(pos)
    }

    pub fn data_size(&mut self) -> Result<u64> {
        let pos = self.fp_data.seek(SeekFrom::End(0))?;
        Ok(pos)
    }

    // Private helper methods
    fn read_bucket(&mut self, bucket: u32) -> Result<MdbPtr> {
        let offset: u64 = (MDB_PTR_SIZE as u64) * (bucket as u64 + 1);
        self.fp_index.seek(SeekFrom::Start(offset))?;
        let mut buf = [0u8; MDB_PTR_SIZE];
        self.fp_index.read_exact(&mut buf)?;
        Ok(MdbPtr::from_le_bytes(buf))
    }

    fn read_index(&mut self, idxptr: MdbPtr) -> Result<MdbIndex> {
        self.fp_index.seek(SeekFrom::Start(idxptr as u64))?;

        let mut next_ptr_buf = [0u8; MDB_PTR_SIZE];
        self.fp_index.read_exact(&mut next_ptr_buf)?;
        let next_ptr = MdbPtr::from_le_bytes(next_ptr_buf);

        let key_size_max = self.options.key_size_max as usize;
        let mut key_buf = vec![0u8; key_size_max];
        self.fp_index.read_exact(&mut key_buf)?;

        let mut value_ptr_buf = [0u8; MDB_PTR_SIZE];
        self.fp_index.read_exact(&mut value_ptr_buf)?;
        let value_ptr = MdbPtr::from_le_bytes(value_ptr_buf);

        let mut value_size_buf = [0u8; MDB_DATALEN_SIZE];
        self.fp_index.read_exact(&mut value_size_buf)?;
        let value_size = MdbSize::from_le_bytes(value_size_buf);

        Ok(MdbIndex {
            next_ptr,
            value_ptr,
            value_size,
            key: key_buf,
        })
    }

    fn write_bucket(&mut self, bucket: u32, ptr: MdbPtr) -> Result<()> {
        let offset: u64 = (MDB_PTR_SIZE as u64) * (bucket as u64 + 1);
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
        // Seek past next_ptr
        self.fp_index
            .seek(SeekFrom::Start(idxptr as u64 + MDB_PTR_SIZE as u64))?;
        // Write key bytes (only key.len(), not key_size_max)
        self.fp_index.write_all(key)?;
        // Seek to value_ptr position
        let value_ptr_pos =
            idxptr as u64 + MDB_PTR_SIZE as u64 + self.options.key_size_max as u64;
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
        if buf.len() < (valsize as usize) + 1 {
            return Err(MdbError::BufferSizeTooSmall);
        }
        self.fp_data.seek(SeekFrom::Start(valptr as u64))?;
        let valsize_us = valsize as usize;
        self.fp_data.read_exact(&mut buf[..valsize_us])?;
        // Null-terminate, matching C behavior.
        buf[valsize_us] = 0;
        Ok(valsize_us)
    }

    fn write_data(&mut self, valptr: MdbPtr, value: &[u8], valsize: MdbSize) -> Result<()> {
        self.fp_data.seek(SeekFrom::Start(valptr as u64))?;
        let valsize_us = valsize as usize;
        self.fp_data.write_all(&value[..valsize_us])?;
        self.fp_data.flush()?;
        Ok(())
    }

    fn stretch_index_file(&mut self, ptr: &mut MdbPtr) -> Result<()> {
        let end_pos = self.fp_index.seek(SeekFrom::End(0))?;
        *ptr = end_pos as MdbPtr;
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
        // Replicate the C scanning algorithm to find a free zero-region in the
        // data file large enough to hold (size + 2) bytes (1 zero byte buffer
        // before the new data and at least 1 zero byte after).
        self.fp_data.seek(SeekFrom::Start(0))?;

        let mut byte_buf = [0u8; 1];
        let mut eof = false;

        loop {
            if eof {
                break;
            }

            // Read first byte of this iteration.
            match self.fp_data.read(&mut byte_buf) {
                Ok(0) => {
                    eof = true;
                }
                Ok(_) => {}
                Err(e) => return Err(MdbError::Io(e)),
            }

            // Skip non-zero bytes.
            while !eof && byte_buf[0] != 0 {
                match self.fp_data.read(&mut byte_buf) {
                    Ok(0) => {
                        eof = true;
                    }
                    Ok(_) => {}
                    Err(e) => return Err(MdbError::Io(e)),
                }
            }

            let start_ptr = self.fp_data.stream_position()? as MdbPtr;

            // Skip zero bytes.
            while !eof && byte_buf[0] == 0 {
                match self.fp_data.read(&mut byte_buf) {
                    Ok(0) => {
                        eof = true;
                    }
                    Ok(_) => {}
                    Err(e) => return Err(MdbError::Io(e)),
                }
            }

            let end_ptr = self.fp_data.stream_position()? as MdbPtr;

            if end_ptr.saturating_sub(start_ptr) >= size + 2 {
                *ptr = start_ptr + 1;
                return Ok(());
            }
        }

        // Append at end of file.
        let end_ptr = self.fp_data.stream_position()? as MdbPtr;
        let zeros = vec![0u8; size as usize];
        self.fp_data.write_all(&zeros)?;
        self.fp_data.flush()?;
        *ptr = end_ptr;
        Ok(())
    }

    fn index_free(&mut self, ptr: MdbPtr) -> Result<()> {
        // Read current free pointer
        self.fp_index.seek(SeekFrom::Start(0))?;
        let mut freeptr_buf = [0u8; MDB_PTR_SIZE];
        self.fp_index.read_exact(&mut freeptr_buf)?;
        let freeptr = MdbPtr::from_le_bytes(freeptr_buf);

        // Set free pointer to ptr
        self.fp_index.seek(SeekFrom::Start(0))?;
        self.fp_index.write_all(&ptr.to_le_bytes())?;

        // Write old freeptr at ptr (link)
        self.fp_index.seek(SeekFrom::Start(ptr as u64))?;
        self.fp_index.write_all(&freeptr.to_le_bytes())?;

        // Clear key part of index (key_size_max zeros)
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
        for (i, b) in key.bytes().enumerate() {
            // Replicate C behavior: signed-char promotion to uint32_t
            let c = b as i8 as i32 as u32;
            ret = ret.wrapping_add(c.wrapping_mul(i as u32));
        }
        ret
    }

    fn close(&mut self) -> Result<()> {
        // Files are closed automatically when the struct is dropped.
        // Flush all to disk for safety.
        let _ = self.fp_superblock.flush();
        let _ = self.fp_index.flush();
        let _ = self.fp_data.flush();
        Ok(())
    }
} // impl Mdb

pub fn mdb_status() -> Result<MdbStatus> {
    Ok(MdbStatus {
        code: MdbStatusCode::MDB_OK as u8,
        desc: String::new(),
    })
}

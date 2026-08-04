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

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut os_str = path.as_os_str().to_os_string();
    os_str.push(suffix);
    PathBuf::from(os_str)
}

fn invalid_data(msg: &str) -> MdbError {
    MdbError::Io(io::Error::new(io::ErrorKind::InvalidData, msg.to_string()))
}

impl Mdb {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let p = path.as_ref();
        let super_path = with_suffix(p, ".db.super");
        let index_path = with_suffix(p, ".db.index");
        let data_path = with_suffix(p, ".db.data");

        let mut fp_superblock = OpenOptions::new()
            .read(true)
            .open(&super_path)?;

        let mut content = String::new();
        fp_superblock.read_to_string(&mut content)?;

        let mut tokens = content.split_whitespace();
        let db_name = tokens
            .next()
            .ok_or_else(|| invalid_data("missing db_name in superblock"))?
            .to_string();
        let key_size_max: u16 = tokens
            .next()
            .ok_or_else(|| invalid_data("missing key_size_max"))?
            .parse()
            .map_err(|_| invalid_data("invalid key_size_max"))?;
        let data_size_max: u32 = tokens
            .next()
            .ok_or_else(|| invalid_data("missing data_size_max"))?
            .parse()
            .map_err(|_| invalid_data("invalid data_size_max"))?;
        let hash_buckets: u32 = tokens
            .next()
            .ok_or_else(|| invalid_data("missing hash_buckets"))?
            .parse()
            .map_err(|_| invalid_data("invalid hash_buckets"))?;
        let items_max: u32 = tokens
            .next()
            .ok_or_else(|| invalid_data("missing items_max"))?
            .parse()
            .map_err(|_| invalid_data("invalid items_max"))?;

        let options = MdbOptions {
            db_name: db_name.clone(),
            key_size_max,
            data_size_max,
            hash_buckets,
            items_max,
        };

        let index_record_size = key_size_max as u32
            + (MDB_PTR_SIZE as u32) * 2
            + MDB_DATALEN_SIZE as u32;

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
        let p = path.as_ref();
        let super_path = with_suffix(p, ".db.super");
        let index_path = with_suffix(p, ".db.index");
        let data_path = with_suffix(p, ".db.data");

        // Validate db_name length (DB_NAME_MAX is 128, mirroring buffer in C).
        if options.db_name.len() > DB_NAME_MAX {
            return Err(MdbError::AllocationFailed);
        }

        let db_name = options.db_name.clone();

        let index_record_size = options.key_size_max as u32
            + (MDB_PTR_SIZE as u32) * 2
            + MDB_DATALEN_SIZE as u32;

        // Open and write the superblock as a text file.
        let mut fp_superblock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&super_path)?;
        let sb_text = format!(
            "{}\n{}\n{}\n{}\n{}\n",
            db_name,
            options.key_size_max,
            options.data_size_max,
            options.hash_buckets,
            options.items_max
        );
        fp_superblock.write_all(sb_text.as_bytes())?;
        fp_superblock.flush()?;

        // Open the index file (truncate) and write the freeptr + buckets.
        let mut fp_index = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&index_path)?;
        let zero_ptr_bytes = (0u32).to_le_bytes();
        // freeptr at offset 0
        fp_index.write_all(&zero_ptr_bytes)?;
        // hash buckets, all zero
        for _ in 0..options.hash_buckets {
            fp_index.write_all(&zero_ptr_bytes)?;
        }
        fp_index.flush()?;

        // Open the data file (truncate, empty).
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
        if self.options.hash_buckets == 0 {
            return Err(MdbError::KeyNotFound);
        }
        let bucket = self.hash(key) % self.options.hash_buckets;
        let mut ptr = self.read_bucket(bucket)?;

        let key_bytes = key.as_bytes();
        while ptr != 0 {
            let index = self.read_index(ptr)?;
            if Self::keys_equal(&index.key, key_bytes) {
                return self.read_data(index.value_ptr, index.value_size, buf);
            }
            ptr = index.next_ptr;
        }

        Err(MdbError::KeyNotFound)
    }

    pub fn write(&mut self, key: &str, value: &str) -> Result<()> {
        let key_bytes = key.as_bytes();
        let value_bytes = value.as_bytes();

        if key_bytes.len() > self.options.key_size_max as usize {
            return Err(MdbError::KeySizeTooLarge);
        }
        if value_bytes.len() > self.options.data_size_max as usize {
            return Err(MdbError::ValueSizeTooLarge);
        }
        if self.options.hash_buckets == 0 {
            return Err(MdbError::AllocationFailed);
        }

        let bucket = self.hash(key) % self.options.hash_buckets;
        let value_size = value_bytes.len() as u32;

        let mut save_ptr: MdbPtr = (MDB_PTR_SIZE as u32) * (bucket + 1);
        let mut ptr = self.read_bucket(bucket)?;

        let mut existing: Option<MdbIndex> = None;
        while ptr != 0 {
            let index = self.read_index(ptr)?;
            if Self::keys_equal(&index.key, key_bytes) {
                existing = Some(index);
                break;
            }
            save_ptr = ptr;
            ptr = index.next_ptr;
        }

        match existing {
            None => {
                // New entry: alloc index, alloc data, write data, write index, link.
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

                if let Err(e) =
                    self.write_index(index_ptr, key_bytes, value_ptr, value_size)
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
            }
            Some(idx) => {
                // Update existing entry: free old data, alloc new, write, update index.
                self.data_free(idx.value_ptr, idx.value_size)?;
                let mut value_ptr: MdbPtr = 0;
                self.data_alloc(value_size, &mut value_ptr)?;
                self.write_data(value_ptr, value_bytes, value_size)?;
                self.write_index(ptr, key_bytes, value_ptr, value_size)?;
                Ok(())
            }
        }
    }

    pub fn delete(&mut self, key: &str) -> Result<()> {
        if self.options.hash_buckets == 0 {
            return Err(MdbError::KeyNotFound);
        }

        let bucket = self.hash(key) % self.options.hash_buckets;
        let key_bytes = key.as_bytes();

        let mut save_ptr: MdbPtr = (MDB_PTR_SIZE as u32) * (bucket + 1);
        let mut ptr = self.read_bucket(bucket)?;

        let mut found: Option<MdbIndex> = None;
        while ptr != 0 {
            let index = self.read_index(ptr)?;
            if Self::keys_equal(&index.key, key_bytes) {
                found = Some(index);
                break;
            }
            save_ptr = ptr;
            ptr = index.next_ptr;
        }

        let Some(idx) = found else {
            return Err(MdbError::KeyNotFound);
        };

        self.data_free(idx.value_ptr, idx.value_size)?;
        self.index_free(ptr)?;
        self.write_nextptr(save_ptr, idx.next_ptr)?;
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
        let offset = (MDB_PTR_SIZE as u64) * (bucket as u64 + 1);
        self.fp_index.seek(SeekFrom::Start(offset))?;
        let mut buf = [0u8; 4];
        self.fp_index.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    fn read_index(&mut self, idxptr: MdbPtr) -> Result<MdbIndex> {
        self.fp_index.seek(SeekFrom::Start(idxptr as u64))?;
        let mut buf4 = [0u8; 4];
        self.fp_index.read_exact(&mut buf4)?;
        let next_ptr = u32::from_le_bytes(buf4);

        let mut key = vec![0u8; self.options.key_size_max as usize];
        self.fp_index.read_exact(&mut key)?;

        self.fp_index.read_exact(&mut buf4)?;
        let value_ptr = u32::from_le_bytes(buf4);

        self.fp_index.read_exact(&mut buf4)?;
        let value_size = u32::from_le_bytes(buf4);

        Ok(MdbIndex {
            next_ptr,
            value_ptr,
            value_size,
            key,
        })
    }

    fn write_bucket(&mut self, bucket: u32, ptr: MdbPtr) -> Result<()> {
        let offset = (MDB_PTR_SIZE as u64) * (bucket as u64 + 1);
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
        // Seek past next_ptr to the key field, write the key bytes (no padding).
        let key_field_start = idxptr as u64 + MDB_PTR_SIZE as u64;
        self.fp_index.seek(SeekFrom::Start(key_field_start))?;
        self.fp_index.write_all(key)?;

        // Seek to value_ptr position (idxptr + ptr_size + key_size_max).
        let value_ptr_pos = idxptr as u64
            + MDB_PTR_SIZE as u64
            + self.options.key_size_max as u64;
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
        Ok(u32::from_le_bytes(buf))
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
        let needed = (valsize as usize).saturating_add(1);
        if buf.len() < needed {
            return Err(MdbError::BufferSizeTooSmall);
        }
        self.fp_data.seek(SeekFrom::Start(valptr as u64))?;
        let valsize_us = valsize as usize;
        if valsize_us > 0 {
            self.fp_data.read_exact(&mut buf[..valsize_us])?;
        }
        // Null-terminate the buffer like the C code does.
        if valsize_us < buf.len() {
            buf[valsize_us] = 0;
        }
        Ok(valsize_us)
    }

    fn write_data(
        &mut self,
        valptr: MdbPtr,
        value: &[u8],
        valsize: MdbSize,
    ) -> Result<()> {
        self.fp_data.seek(SeekFrom::Start(valptr as u64))?;
        let valsize_us = valsize as usize;
        if valsize_us > 0 {
            self.fp_data.write_all(&value[..valsize_us])?;
        }
        self.fp_data.flush()?;
        Ok(())
    }

    fn stretch_index_file(&mut self, ptr: &mut MdbPtr) -> Result<()> {
        let end = self.fp_index.seek(SeekFrom::End(0))?;
        *ptr = end as u32;
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
        // Read entire data file and scan for a usable zero region.
        self.fp_data.seek(SeekFrom::Start(0))?;
        let mut data = Vec::new();
        self.fp_data.read_to_end(&mut data)?;
        let len = data.len();

        // Mirror the C scan algorithm precisely. We track:
        //   pos  - the next byte index to read (analog of ftell after reads)
        //   byte - the last byte successfully read
        //   feof - whether we've attempted to read past end of file
        let mut pos: usize = 0;
        let mut byte: u8 = 0;
        let mut feof = false;

        // Helper inline: read one byte from the buffer.
        // The C does an extra fread before each inner loop pair.
        while !feof {
            // Initial fread() at the top of the outer while body.
            if pos < len {
                byte = data[pos];
                pos += 1;
            } else {
                feof = true;
            }
            // Skip non-zero bytes.
            while !feof && byte != 0 {
                if pos < len {
                    byte = data[pos];
                    pos += 1;
                } else {
                    feof = true;
                }
            }
            let start_ptr = pos as u32;
            // Skip zero bytes.
            while !feof && byte == 0 {
                if pos < len {
                    byte = data[pos];
                    pos += 1;
                } else {
                    feof = true;
                }
            }
            let end_ptr = pos as u32;

            if end_ptr.wrapping_sub(start_ptr) >= size.wrapping_add(2) {
                *ptr = start_ptr + 1;
                return Ok(());
            }
        }

        // No suitable hole — extend the file with `size` zero bytes.
        let end_ptr = pos as u32;
        self.fp_data.seek(SeekFrom::End(0))?;
        let zeros = vec![0u8; size as usize];
        if !zeros.is_empty() {
            self.fp_data.write_all(&zeros)?;
            self.fp_data.flush()?;
        }
        *ptr = end_ptr;
        Ok(())
    }

    fn index_free(&mut self, ptr: MdbPtr) -> Result<()> {
        // Read current freeptr from offset 0.
        self.fp_index.seek(SeekFrom::Start(0))?;
        let mut buf = [0u8; 4];
        self.fp_index.read_exact(&mut buf)?;
        let freeptr = u32::from_le_bytes(buf);

        // Set freeptr at offset 0 to ptr.
        self.fp_index.seek(SeekFrom::Start(0))?;
        self.fp_index.write_all(&ptr.to_le_bytes())?;

        // At ptr, write old freeptr as next_ptr, then zero out the key field.
        self.fp_index.seek(SeekFrom::Start(ptr as u64))?;
        self.fp_index.write_all(&freeptr.to_le_bytes())?;

        let zeros = vec![0u8; self.options.key_size_max as usize];
        if !zeros.is_empty() {
            self.fp_index.write_all(&zeros)?;
        }

        self.fp_index.flush()?;
        Ok(())
    }

    fn data_free(&mut self, ptr: MdbPtr, size: MdbSize) -> Result<()> {
        self.fp_data.seek(SeekFrom::Start(ptr as u64))?;
        let zeros = vec![0u8; size as usize];
        if !zeros.is_empty() {
            self.fp_data.write_all(&zeros)?;
            self.fp_data.flush()?;
        }
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
        for (i, &b) in key.as_bytes().iter().enumerate() {
            // Match the C behavior of `(uint32_t)*key * i` using wrapping ops.
            let term = (b as u32).wrapping_mul(i as u32);
            ret = ret.wrapping_add(term);
        }
        ret
    }

    fn close(&mut self) -> Result<()> {
        self.fp_superblock.flush()?;
        self.fp_index.flush()?;
        self.fp_data.flush()?;
        Ok(())
    }

    // Compare a stored key (NUL-padded byte slice of length key_size_max)
    // with a candidate key (raw byte slice).
    fn keys_equal(stored: &[u8], candidate: &[u8]) -> bool {
        // Find first NUL in stored (or use full length).
        let stored_len = stored
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(stored.len());
        &stored[..stored_len] == candidate
    }
} // impl Mdb

pub fn mdb_status() -> Result<MdbStatus> {
    Ok(MdbStatus {
        code: MdbStatusCode::MDB_OK as u8,
        desc: String::new(),
    })
}

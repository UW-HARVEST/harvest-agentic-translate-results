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

fn make_paths(path: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let s = path.as_os_str().to_string_lossy().into_owned();
    (
        PathBuf::from(format!("{}.super", s)),
        PathBuf::from(format!("{}.index", s)),
        PathBuf::from(format!("{}.data", s)),
    )
}

impl Mdb {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let (super_path, index_path, data_path) = make_paths(path);

        let mut fp_superblock = File::open(&super_path)?;
        let mut superblock_content = String::new();
        fp_superblock.read_to_string(&mut superblock_content)?;

        // Parse the superblock: db_name, key_size_max, data_size_max, hash_buckets, items_max
        // Format: each field separated by whitespace (newline in C). fscanf with %s reads until whitespace.
        let mut tokens = superblock_content.split_whitespace();
        let db_name = tokens
            .next()
            .ok_or(MdbError::Io(io::Error::new(
                io::ErrorKind::InvalidData,
                "missing db_name in superblock",
            )))?
            .to_string();
        let key_size_max: u16 = tokens
            .next()
            .ok_or(MdbError::Io(io::Error::new(
                io::ErrorKind::InvalidData,
                "missing key_size_max",
            )))?
            .parse()
            .map_err(|_| {
                MdbError::Io(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "bad key_size_max",
                ))
            })?;
        let data_size_max: u32 = tokens
            .next()
            .ok_or(MdbError::Io(io::Error::new(
                io::ErrorKind::InvalidData,
                "missing data_size_max",
            )))?
            .parse()
            .map_err(|_| {
                MdbError::Io(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "bad data_size_max",
                ))
            })?;
        let hash_buckets: u32 = tokens
            .next()
            .ok_or(MdbError::Io(io::Error::new(
                io::ErrorKind::InvalidData,
                "missing hash_buckets",
            )))?
            .parse()
            .map_err(|_| {
                MdbError::Io(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "bad hash_buckets",
                ))
            })?;
        let items_max: u32 = tokens
            .next()
            .ok_or(MdbError::Io(io::Error::new(
                io::ErrorKind::InvalidData,
                "missing items_max",
            )))?
            .parse()
            .map_err(|_| {
                MdbError::Io(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "bad items_max",
                ))
            })?;

        let options = MdbOptions {
            db_name: db_name.clone(),
            key_size_max,
            data_size_max,
            hash_buckets,
            items_max,
        };
        let index_record_size =
            options.key_size_max as u32 + (MDB_PTR_SIZE * 2) as u32 + MDB_DATALEN_SIZE as u32;

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
        let path = path.as_ref();
        let (super_path, index_path, data_path) = make_paths(path);

        let db_name = options.db_name.clone();
        let index_record_size =
            options.key_size_max as u32 + (MDB_PTR_SIZE * 2) as u32 + MDB_DATALEN_SIZE as u32;

        // Open/create superblock for writing (truncate if exists)
        let mut fp_superblock = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .read(true)
            .open(&super_path)?;
        // Write superblock: db_name, key_size_max, data_size_max, hash_buckets, items_max
        let content = format!(
            "{}\n{}\n{}\n{}\n{}\n",
            db_name,
            options.key_size_max,
            options.data_size_max,
            options.hash_buckets,
            options.items_max
        );
        fp_superblock.write_all(content.as_bytes())?;
        fp_superblock.flush()?;

        // Open/create index file with read+write+truncate
        let mut fp_index = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&index_path)?;
        // Write zero pointer for freeptr, then zero pointers for each hash bucket
        let zero = [0u8; MDB_PTR_SIZE];
        fp_index.write_all(&zero)?; // freeptr
        for _ in 0..options.hash_buckets {
            fp_index.write_all(&zero)?;
        }
        fp_index.flush()?;

        // Open/create data file with read+write+truncate
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
            // Compare keys: index.key is null-padded; find null
            let key_str = bytes_to_str(&index.key);
            if key_str == key {
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
        let value_size = value.len() as u32;
        if value_size > self.options.data_size_max {
            return Err(MdbError::ValueSizeTooLarge);
        }

        // save_ptr starts as the position in the index file that holds the
        // bucket head pointer (offset MDB_PTR_SIZE * (bucket + 1)).
        let mut save_ptr: MdbPtr = (MDB_PTR_SIZE as u32) * (bucket + 1);
        let mut ptr = self.read_bucket(bucket)?;

        let mut found_index: Option<MdbIndex> = None;
        while ptr != 0 {
            let index = self.read_index(ptr)?;
            let key_str = bytes_to_str(&index.key);
            if key_str == key {
                found_index = Some(index);
                break;
            }
            save_ptr = ptr;
            ptr = index.next_ptr;
        }

        if ptr == 0 {
            // Insert new entry
            let mut index_ptr: MdbPtr = 0;
            self.index_alloc(&mut index_ptr)?;
            let mut value_ptr: MdbPtr = 0;
            match self.data_alloc(value_size, &mut value_ptr) {
                Ok(()) => {}
                Err(e) => {
                    let _ = self.index_free(index_ptr);
                    return Err(e);
                }
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
            // Update existing entry
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
            let key_str = bytes_to_str(&index.key);
            if key_str == key {
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
        let offset = (MDB_PTR_SIZE as u64) * (bucket as u64 + 1);
        self.fp_index.seek(SeekFrom::Start(offset))?;
        let mut buf = [0u8; MDB_PTR_SIZE];
        self.fp_index.read_exact(&mut buf)?;
        Ok(MdbPtr::from_le_bytes(buf))
    }

    fn read_index(&mut self, idxptr: MdbPtr) -> Result<MdbIndex> {
        self.fp_index.seek(SeekFrom::Start(idxptr as u64))?;
        let mut nextbuf = [0u8; MDB_PTR_SIZE];
        self.fp_index.read_exact(&mut nextbuf)?;
        let next_ptr = MdbPtr::from_le_bytes(nextbuf);

        let mut key = vec![0u8; self.options.key_size_max as usize];
        self.fp_index.read_exact(&mut key)?;

        let mut vpbuf = [0u8; MDB_PTR_SIZE];
        self.fp_index.read_exact(&mut vpbuf)?;
        let value_ptr = MdbPtr::from_le_bytes(vpbuf);

        let mut vsbuf = [0u8; MDB_DATALEN_SIZE];
        self.fp_index.read_exact(&mut vsbuf)?;
        let value_size = MdbSize::from_le_bytes(vsbuf);

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
        // Seek to idxptr + MDB_PTR_SIZE and write the key bytes (only key.len(), not padded).
        self.fp_index
            .seek(SeekFrom::Start(idxptr as u64 + MDB_PTR_SIZE as u64))?;
        self.fp_index.write_all(key)?;
        // Seek to value_ptr position
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
        if (buf.len() as u64) < (valsize as u64) + 1 {
            return Err(MdbError::BufferSizeTooSmall);
        }
        self.fp_data.seek(SeekFrom::Start(valptr as u64))?;
        let dst = &mut buf[..valsize as usize];
        self.fp_data.read_exact(dst)?;
        // Match C behavior: null-terminate at valsize
        if (valsize as usize) < buf.len() {
            buf[valsize as usize] = 0;
        }
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
        // Walk the data file looking for a contiguous run of zero bytes of
        // at least size+2 bytes (1 leading non-zero byte + size zero bytes +
        // 1 trailing non-zero byte). Allocate at start_ptr+1.
        self.fp_data.seek(SeekFrom::Start(0))?;

        // Read entire data file into memory (typical sizes are small enough)
        let mut content = Vec::new();
        self.fp_data.read_to_end(&mut content)?;

        let mut i: usize = 0;
        let n = content.len();
        loop {
            // Skip non-zero bytes (these include the file start, since we treat
            // pre-existing data as occupied)
            while i < n && content[i] != 0 {
                i += 1;
            }
            // Now position i points to a zero byte (or EOF). In the C
            // implementation, start_ptr is recorded after reading the
            // non-zero byte (so it's the position right after it, which is
            // the index of the first zero byte).
            let start_ptr = i;
            // Skip zero bytes until we hit a non-zero byte (or EOF)
            while i < n && content[i] == 0 {
                i += 1;
            }
            // After this, i is at the first non-zero byte OR at EOF.
            // The C code records end_ptr after reading the non-zero byte, so
            // end_ptr is (position of non-zero byte) + 1.
            let end_ptr = if i < n { i + 1 } else { i };

            // Need at least size + 2 bytes between start_ptr and end_ptr
            if (end_ptr as u32).saturating_sub(start_ptr as u32) >= size + 2 {
                *ptr = (start_ptr + 1) as MdbPtr;
                return Ok(());
            }

            // If we've reached EOF, exit and fall through to stretching
            if i >= n {
                break;
            }
            // Otherwise advance past the non-zero byte
            i = end_ptr;
        }

        // Couldn't find a free slot - extend the file
        let end_ptr = self.fp_data.seek(SeekFrom::End(0))?;
        let zeros = vec![0u8; size as usize];
        self.fp_data.write_all(&zeros)?;
        self.fp_data.flush()?;
        *ptr = end_ptr as MdbPtr;
        Ok(())
    }

    fn index_free(&mut self, ptr: MdbPtr) -> Result<()> {
        // Read freeptr from offset 0
        self.fp_index.seek(SeekFrom::Start(0))?;
        let mut buf = [0u8; MDB_PTR_SIZE];
        self.fp_index.read_exact(&mut buf)?;
        let freeptr = MdbPtr::from_le_bytes(buf);

        // Write ptr to freeptr position (offset 0)
        self.fp_index.seek(SeekFrom::Start(0))?;
        self.fp_index.write_all(&ptr.to_le_bytes())?;

        // Seek to ptr and write the old freeptr there (as new next_ptr of ptr)
        self.fp_index.seek(SeekFrom::Start(ptr as u64))?;
        self.fp_index.write_all(&freeptr.to_le_bytes())?;

        // Clear the key portion of the freed slot
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
            ret = ret.wrapping_add((b as u32).wrapping_mul(i as u32));
        }
        ret
    }

    fn close(&mut self) -> Result<()> {
        self.fp_superblock.flush()?;
        self.fp_index.flush()?;
        self.fp_data.flush()?;
        Ok(())
    }
} // impl Mdb

fn bytes_to_str(bytes: &[u8]) -> &str {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    std::str::from_utf8(&bytes[..end]).unwrap_or("")
}

pub fn mdb_status() -> Result<MdbStatus> {
    Ok(MdbStatus {
        code: MdbStatusCode::MDB_OK as u8,
        desc: String::new(),
    })
}

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

fn make_path<P: AsRef<Path>>(base: P, suffix: &str) -> PathBuf {
    let mut s = base.as_ref().as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

impl Mdb {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let super_path = make_path(&path, ".db.super");
        let super_file = File::open(&super_path)?;
        let mut reader = BufReader::new(&super_file);

        let mut db_name = String::new();
        reader.read_line(&mut db_name)?;
        let db_name = db_name.trim().to_string();

        let mut line = String::new();
        reader.read_line(&mut line)?;
        let key_size_max: u16 = line.trim().parse().map_err(|_| {
            MdbError::Io(io::Error::new(io::ErrorKind::InvalidData, "key_size_max"))
        })?;

        line.clear();
        reader.read_line(&mut line)?;
        let data_size_max: u32 = line.trim().parse().map_err(|_| {
            MdbError::Io(io::Error::new(io::ErrorKind::InvalidData, "data_size_max"))
        })?;

        line.clear();
        reader.read_line(&mut line)?;
        let hash_buckets: u32 = line.trim().parse().map_err(|_| {
            MdbError::Io(io::Error::new(io::ErrorKind::InvalidData, "hash_buckets"))
        })?;

        line.clear();
        reader.read_line(&mut line)?;
        let items_max: u32 = line.trim().parse().map_err(|_| {
            MdbError::Io(io::Error::new(io::ErrorKind::InvalidData, "items_max"))
        })?;

        drop(reader);

        let options = MdbOptions {
            db_name: db_name.clone(),
            key_size_max,
            data_size_max,
            hash_buckets,
            items_max,
        };

        let index_record_size: u32 =
            (key_size_max as u32) + (MDB_PTR_SIZE as u32) * 2 + (MDB_DATALEN_SIZE as u32);

        let index_path = make_path(&path, ".db.index");
        let fp_index = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&index_path)?;

        let data_path = make_path(&path, ".db.data");
        let fp_data = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&data_path)?;

        Ok(Mdb {
            db_name,
            fp_superblock: super_file,
            fp_index,
            fp_data,
            options,
            index_record_size,
        })
    }

    pub fn create<P: AsRef<Path>>(path: P, options: MdbOptions) -> Result<Self> {
        if options.db_name.len() > DB_NAME_MAX {
            return Err(MdbError::AllocationFailed);
        }

        let index_record_size: u32 = (options.key_size_max as u32)
            + (MDB_PTR_SIZE as u32) * 2
            + (MDB_DATALEN_SIZE as u32);

        let super_path = make_path(&path, ".db.super");
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

        let index_path = make_path(&path, ".db.index");
        let mut fp_index = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&index_path)?;

        let zero_ptr_bytes = [0u8; MDB_PTR_SIZE];
        // Free pointer at start.
        fp_index.write_all(&zero_ptr_bytes)?;
        // Hash buckets, all zero.
        for _ in 0..options.hash_buckets {
            fp_index.write_all(&zero_ptr_bytes)?;
        }
        fp_index.flush()?;

        let data_path = make_path(&path, ".db.data");
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
            if Self::key_eq(&index.key, key.as_bytes()) {
                return self.read_data(index.value_ptr, index.value_size, buf);
            }
            ptr = index.next_ptr;
        }
        Err(MdbError::KeyNotFound)
    }

    pub fn write(&mut self, key: &str, value: &str) -> Result<()> {
        let bucket = self.hash(key) % self.options.hash_buckets;
        let key_bytes = key.as_bytes();
        let key_size = key_bytes.len();
        if key_size > self.options.key_size_max as usize {
            return Err(MdbError::KeySizeTooLarge);
        }
        let value_bytes = value.as_bytes();
        let value_size = value_bytes.len() as u32;
        if value_size > self.options.data_size_max {
            return Err(MdbError::ValueSizeTooLarge);
        }

        let mut save_ptr: MdbPtr = (MDB_PTR_SIZE as u32) * (bucket + 1);
        let mut ptr = self.read_bucket(bucket)?;

        let mut found_index: Option<MdbIndex> = None;
        while ptr != 0 {
            let index = self.read_index(ptr)?;
            if Self::key_eq(&index.key, key_bytes) {
                found_index = Some(index);
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
            match self.data_alloc(value_size, &mut value_ptr) {
                Ok(()) => {}
                Err(e) => {
                    let _ = self.index_free(index_ptr);
                    return Err(e);
                }
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
            // Update existing entry
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

        let mut save_ptr: MdbPtr = (MDB_PTR_SIZE as u32) * (bucket + 1);
        let mut ptr = self.read_bucket(bucket)?;

        let mut found_index: Option<MdbIndex> = None;
        while ptr != 0 {
            let index = self.read_index(ptr)?;
            if Self::key_eq(&index.key, key_bytes) {
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
        let offset = (MDB_PTR_SIZE as u64) * ((bucket as u64) + 1);
        self.fp_index.seek(SeekFrom::Start(offset))?;
        let mut buf = [0u8; MDB_PTR_SIZE];
        self.fp_index.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    fn read_index(&mut self, idxptr: MdbPtr) -> Result<MdbIndex> {
        self.fp_index.seek(SeekFrom::Start(idxptr as u64))?;

        let mut nbuf = [0u8; MDB_PTR_SIZE];
        self.fp_index.read_exact(&mut nbuf)?;
        let next_ptr = u32::from_le_bytes(nbuf);

        let key_size_max = self.options.key_size_max as usize;
        let mut key = vec![0u8; key_size_max];
        self.fp_index.read_exact(&mut key)?;

        let mut vbuf = [0u8; MDB_PTR_SIZE];
        self.fp_index.read_exact(&mut vbuf)?;
        let value_ptr = u32::from_le_bytes(vbuf);

        let mut sbuf = [0u8; MDB_DATALEN_SIZE];
        self.fp_index.read_exact(&mut sbuf)?;
        let value_size = u32::from_le_bytes(sbuf);

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
        // Write the key starting at idxptr + MDB_PTR_SIZE (after the next_ptr).
        let key_offset = (idxptr as u64) + (MDB_PTR_SIZE as u64);
        self.fp_index.seek(SeekFrom::Start(key_offset))?;
        // Write only key bytes (no null terminator), matching C strlen behavior.
        self.fp_index.write_all(key)?;

        // Write value_ptr and value_size at idxptr + MDB_PTR_SIZE + key_size_max.
        let value_pos = (idxptr as u64)
            + (MDB_PTR_SIZE as u64)
            + (self.options.key_size_max as u64);
        self.fp_index.seek(SeekFrom::Start(value_pos))?;
        self.fp_index.write_all(&value_ptr.to_le_bytes())?;
        self.fp_index.write_all(&value_size.to_le_bytes())?;
        self.fp_index.flush()?;
        Ok(())
    }

    fn read_nextptr(&mut self, idxptr: MdbPtr) -> Result<MdbPtr> {
        self.fp_index.seek(SeekFrom::Start(idxptr as u64))?;
        let mut buf = [0u8; MDB_PTR_SIZE];
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
        if (buf.len() as u64) < (valsize as u64) + 1 {
            return Err(MdbError::BufferSizeTooSmall);
        }
        self.fp_data.seek(SeekFrom::Start(valptr as u64))?;
        self.fp_data.read_exact(&mut buf[..valsize as usize])?;
        // Null terminator (matching C behavior).
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
        self.fp_data.seek(SeekFrom::Start(valptr as u64))?;
        self.fp_data
            .write_all(&value[..valsize as usize])?;
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
        // Read entire data file, then scan for a free run of zeros.
        self.fp_data.seek(SeekFrom::Start(0))?;
        let mut data: Vec<u8> = Vec::new();
        self.fp_data.read_to_end(&mut data)?;
        let len = data.len();

        let mut pos: usize = 0;
        while pos < len {
            // Outer fread.
            let mut byte = data[pos];
            pos += 1;
            // Inner non-zero loop.
            while pos < len && byte != 0 {
                byte = data[pos];
                pos += 1;
            }
            // EOF reached or hit a zero byte.
            let start_ptr = pos as u32;
            // Inner zero loop.
            while pos < len && byte == 0 {
                byte = data[pos];
                pos += 1;
            }
            let end_ptr = pos as u32;

            if end_ptr - start_ptr >= size + 2 {
                *ptr = start_ptr + 1;
                return Ok(());
            }
        }

        // Append at end.
        let end_ptr = len as u32;
        let zeros = vec![0u8; size as usize];
        self.fp_data.seek(SeekFrom::End(0))?;
        self.fp_data.write_all(&zeros)?;
        self.fp_data.flush()?;
        *ptr = end_ptr;
        Ok(())
    }

    fn index_free(&mut self, ptr: MdbPtr) -> Result<()> {
        // Read the freeptr at offset 0.
        self.fp_index.seek(SeekFrom::Start(0))?;
        let mut buf = [0u8; MDB_PTR_SIZE];
        self.fp_index.read_exact(&mut buf)?;
        let freeptr = u32::from_le_bytes(buf);

        // Write ptr as the new freeptr at offset 0.
        self.fp_index.seek(SeekFrom::Start(0))?;
        self.fp_index.write_all(&ptr.to_le_bytes())?;

        // Write old freeptr as the next_ptr of this freed record.
        self.fp_index.seek(SeekFrom::Start(ptr as u64))?;
        self.fp_index.write_all(&freeptr.to_le_bytes())?;

        // Zero out the key portion.
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
        // Allocation in Rust is implicit; this is a no-op marker matching the C helper.
        Ok(())
    }

    fn free() -> Result<()> {
        // Deallocation is automatic in Rust; this is a no-op marker matching the C helper.
        Ok(())
    }

    fn hash(&self, key: &str) -> u32 {
        let mut ret: u32 = 0;
        for (i, b) in key.as_bytes().iter().enumerate() {
            ret = ret.wrapping_add((*b as u32).wrapping_mul(i as u32));
        }
        ret
    }

    fn close(&mut self) -> Result<()> {
        self.fp_superblock.flush()?;
        self.fp_index.flush()?;
        self.fp_data.flush()?;
        Ok(())
    }

    /// Compare a fixed-width key buffer (read from disk) to the user-provided
    /// key, treating the disk buffer as null-terminated within `key_size_max`.
    fn key_eq(disk_key: &[u8], target: &[u8]) -> bool {
        // Find null terminator position in disk_key (or end).
        let mut nul = disk_key.len();
        for (i, b) in disk_key.iter().enumerate() {
            if *b == 0 {
                nul = i;
                break;
            }
        }
        &disk_key[..nul] == target
    }
}
pub fn mdb_status() -> Result<MdbStatus> {
    Ok(MdbStatus {
        code: 0,
        desc: String::new(),
    })
}

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

fn db_paths<P: AsRef<Path>>(path: P) -> [PathBuf; 3] {
    let base = path.as_ref();
    [
        base.with_extension("super"),
        base.with_extension("index"),
        base.with_extension("data"),
    ]
}

fn db_paths_with_legacy_suffix<P: AsRef<Path>>(path: P) -> [PathBuf; 3] {
    let base = path.as_ref().as_os_str().to_string_lossy();
    [
        PathBuf::from(format!("{base}.db.super")),
        PathBuf::from(format!("{base}.db.index")),
        PathBuf::from(format!("{base}.db.data")),
    ]
}

fn nul_terminated_len(bytes: &[u8]) -> usize {
    bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len())
}

fn io_invalid_data(msg: &str) -> MdbError {
    MdbError::Io(io::Error::new(io::ErrorKind::InvalidData, msg))
}

fn read_u32(file: &mut File) -> Result<u32> {
    let mut buf = [0u8; 4];
    file.read_exact(&mut buf)?;
    Ok(u32::from_ne_bytes(buf))
}

fn write_u32(file: &mut File, value: u32) -> Result<()> {
    file.write_all(&value.to_ne_bytes())?;
    Ok(())
}

impl Mdb {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let primary = db_paths(&path);
        let legacy = db_paths_with_legacy_suffix(&path);
        let chosen = if primary[0].exists() {
            primary
        } else {
            legacy
        };

        let mut fp_superblock = OpenOptions::new().read(true).open(&chosen[0])?;
        let mut superblock = String::new();
        fp_superblock.read_to_string(&mut superblock)?;

        let mut tokens = superblock.split_whitespace();
        let options = MdbOptions {
            db_name: tokens
                .next()
                .ok_or_else(|| io_invalid_data("missing db_name in superblock"))?
                .to_string(),
            key_size_max: tokens
                .next()
                .ok_or_else(|| io_invalid_data("missing key_size_max in superblock"))?
                .parse()
                .map_err(|_| io_invalid_data("invalid key_size_max in superblock"))?,
            data_size_max: tokens
                .next()
                .ok_or_else(|| io_invalid_data("missing data_size_max in superblock"))?
                .parse()
                .map_err(|_| io_invalid_data("invalid data_size_max in superblock"))?,
            hash_buckets: tokens
                .next()
                .ok_or_else(|| io_invalid_data("missing hash_buckets in superblock"))?
                .parse()
                .map_err(|_| io_invalid_data("invalid hash_buckets in superblock"))?,
            items_max: tokens
                .next()
                .ok_or_else(|| io_invalid_data("missing items_max in superblock"))?
                .parse()
                .map_err(|_| io_invalid_data("invalid items_max in superblock"))?,
        };

        let fp_index = OpenOptions::new().read(true).write(true).open(&chosen[1])?;
        let fp_data = OpenOptions::new().read(true).write(true).open(&chosen[2])?;
        let index_record_size =
            options.key_size_max as u32 + (MDB_PTR_SIZE as u32) * 2 + MDB_DATALEN_SIZE as u32;

        Ok(Self {
            db_name: options.db_name.clone(),
            fp_superblock,
            fp_index,
            fp_data,
            options,
            index_record_size,
        })
    }
    pub fn create<P: AsRef<Path>>(path: P, options: MdbOptions) -> Result<Self> {
        let [super_path, index_path, data_path] = db_paths(path);
        let mut fp_superblock = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .read(true)
            .open(super_path)?;
        writeln!(fp_superblock, "{}", options.db_name)?;
        writeln!(fp_superblock, "{}", options.key_size_max)?;
        writeln!(fp_superblock, "{}", options.data_size_max)?;
        writeln!(fp_superblock, "{}", options.hash_buckets)?;
        writeln!(fp_superblock, "{}", options.items_max)?;
        fp_superblock.flush()?;
        fp_superblock.seek(SeekFrom::Start(0))?;

        let mut fp_index = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .read(true)
            .open(index_path)?;
        write_u32(&mut fp_index, 0)?;
        for _ in 0..options.hash_buckets {
            write_u32(&mut fp_index, 0)?;
        }
        fp_index.flush()?;
        fp_index.seek(SeekFrom::Start(0))?;

        let fp_data = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .read(true)
            .open(data_path)?;

        let index_record_size =
            options.key_size_max as u32 + (MDB_PTR_SIZE as u32) * 2 + MDB_DATALEN_SIZE as u32;

        Ok(Self {
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
            if index.key == key.as_bytes() {
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

        let bucket = self.hash(key) % self.options.hash_buckets;
        let mut save_ptr = MDB_PTR_SIZE as u32 * (bucket + 1);
        let mut ptr = self.read_bucket(bucket)?;

        while ptr != 0 {
            let index = self.read_index(ptr)?;
            if index.key == key.as_bytes() {
                self.data_free(index.value_ptr, index.value_size)?;
                let mut value_ptr = 0;
                self.data_alloc(value.len() as MdbSize, &mut value_ptr)?;
                self.write_data(value_ptr, value.as_bytes(), value.len() as MdbSize)?;
                self.write_index(ptr, key.as_bytes(), value_ptr, value.len() as MdbSize)?;
                return Ok(());
            }
            save_ptr = ptr;
            ptr = index.next_ptr;
        }

        let mut index_ptr = 0;
        self.index_alloc(&mut index_ptr)?;
        let mut value_ptr = 0;
        if let Err(err) = self.data_alloc(value.len() as MdbSize, &mut value_ptr) {
            let _ = self.index_free(index_ptr);
            return Err(err);
        }
        if let Err(err) = self.write_data(value_ptr, value.as_bytes(), value.len() as MdbSize) {
            let _ = self.data_free(value_ptr, value.len() as MdbSize);
            let _ = self.index_free(index_ptr);
            return Err(err);
        }
        if let Err(err) = self.write_index(index_ptr, key.as_bytes(), value_ptr, value.len() as MdbSize) {
            let _ = self.data_free(value_ptr, value.len() as MdbSize);
            let _ = self.index_free(index_ptr);
            return Err(err);
        }
        if let Err(err) = self.write_nextptr(save_ptr, index_ptr) {
            let _ = self.data_free(value_ptr, value.len() as MdbSize);
            let _ = self.index_free(index_ptr);
            return Err(err);
        }
        Ok(())
    }
    pub fn delete(&mut self, key: &str) -> Result<()> {
        let bucket = self.hash(key) % self.options.hash_buckets;
        let mut save_ptr = MDB_PTR_SIZE as u32 * (bucket + 1);
        let mut ptr = self.read_bucket(bucket)?;

        while ptr != 0 {
            let index = self.read_index(ptr)?;
            if index.key == key.as_bytes() {
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
        Ok(self.fp_index.seek(SeekFrom::End(0))?)
    }
    pub fn data_size(&mut self) -> Result<u64> {
        Ok(self.fp_data.seek(SeekFrom::End(0))?)
    }
    // Private helper methods
    fn read_bucket(&mut self, bucket: u32) -> Result<MdbPtr> {
        let offset = (MDB_PTR_SIZE as u64) * (bucket as u64 + 1);
        self.fp_index.seek(SeekFrom::Start(offset))?;
        read_u32(&mut self.fp_index)
    }
    fn read_index(&mut self, idxptr: MdbPtr) -> Result<MdbIndex> {
        self.fp_index.seek(SeekFrom::Start(idxptr as u64))?;
        let next_ptr = read_u32(&mut self.fp_index)?;
        let mut key = vec![0u8; self.options.key_size_max as usize];
        self.fp_index.read_exact(&mut key)?;
        key.truncate(nul_terminated_len(&key));
        let value_ptr = read_u32(&mut self.fp_index)?;
        let value_size = read_u32(&mut self.fp_index)?;

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
        write_u32(&mut self.fp_index, ptr)?;
        self.fp_index.flush()?;
        Ok(())
    }
    fn write_index(&mut self, idxptr: MdbPtr, key: &[u8], value_ptr: MdbPtr, value_size: MdbSize) -> Result<()> {
        self.fp_index
            .seek(SeekFrom::Start(idxptr as u64 + MDB_PTR_SIZE as u64))?;
        self.fp_index.write_all(key)?;
        let value_ptr_pos = idxptr as u64 + MDB_PTR_SIZE as u64 + self.options.key_size_max as u64;
        self.fp_index.seek(SeekFrom::Start(value_ptr_pos))?;
        write_u32(&mut self.fp_index, value_ptr)?;
        write_u32(&mut self.fp_index, value_size)?;
        self.fp_index.flush()?;
        Ok(())
    }
    fn read_nextptr(&mut self, idxptr: MdbPtr) -> Result<MdbPtr> {
        self.fp_index.seek(SeekFrom::Start(idxptr as u64))?;
        read_u32(&mut self.fp_index)
    }
    fn write_nextptr(&mut self, ptr: MdbPtr, nextptr: MdbPtr) -> Result<()> {
        self.fp_index.seek(SeekFrom::Start(ptr as u64))?;
        write_u32(&mut self.fp_index, nextptr)?;
        self.fp_index.flush()?;
        Ok(())
    }
    fn read_data(&mut self, valptr: MdbPtr, valsize: MdbSize, buf: &mut [u8]) -> Result<usize> {
        let valsize = valsize as usize;
        if buf.len() < valsize + 1 {
            return Err(MdbError::BufferSizeTooSmall);
        }
        self.fp_data.seek(SeekFrom::Start(valptr as u64))?;
        self.fp_data.read_exact(&mut buf[..valsize])?;
        buf[valsize] = 0;
        Ok(valsize)
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

        loop {
            let mut byte = [0u8; 1];
            match self.fp_data.read(&mut byte)? {
                0 => break,
                _ => {}
            }

            while byte[0] != 0 {
                if self.fp_data.read(&mut byte)? == 0 {
                    let end_ptr = self.fp_data.stream_position()? as MdbPtr;
                    self.fp_data.seek(SeekFrom::End(0))?;
                    self.fp_data.write_all(&vec![0u8; size as usize])?;
                    self.fp_data.flush()?;
                    *ptr = end_ptr;
                    return Ok(());
                }
            }

            let start_ptr = self.fp_data.stream_position()? as MdbPtr;
            loop {
                if byte[0] != 0 {
                    break;
                }
                if self.fp_data.read(&mut byte)? == 0 {
                    break;
                }
            }
            let end_ptr = self.fp_data.stream_position()? as MdbPtr;

            if end_ptr.saturating_sub(start_ptr) >= size + 2 {
                *ptr = start_ptr + 1;
                return Ok(());
            }

            if byte[0] != 0 {
                continue;
            }
            break;
        }

        *ptr = self.fp_data.seek(SeekFrom::End(0))? as MdbPtr;
        self.fp_data.write_all(&vec![0u8; size as usize])?;
        self.fp_data.flush()?;
        Ok(())
    }
    fn index_free(&mut self, ptr: MdbPtr) -> Result<()> {
        self.fp_index.seek(SeekFrom::Start(0))?;
        let freeptr = read_u32(&mut self.fp_index)?;
        self.fp_index.seek(SeekFrom::Start(0))?;
        write_u32(&mut self.fp_index, ptr)?;
        self.fp_index.seek(SeekFrom::Start(ptr as u64))?;
        write_u32(&mut self.fp_index, freeptr)?;
        self.fp_index
            .write_all(&vec![0u8; self.options.key_size_max as usize])?;
        self.fp_index.flush()?;
        Ok(())
    }
    fn data_free(&mut self, ptr: MdbPtr, size: MdbSize) -> Result<()> {
        self.fp_data.seek(SeekFrom::Start(ptr as u64))?;
        self.fp_data.write_all(&vec![0u8; size as usize])?;
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
        let mut ret = 0u32;
        for (i, byte) in key.bytes().take_while(|&b| b != 0).enumerate() {
            ret = ret.wrapping_add((byte as u32).wrapping_mul(i as u32));
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
pub fn mdb_status() -> Result<MdbStatus> {
    Ok(MdbStatus {
        code: MdbStatusCode::MDB_OK as u8,
        desc: String::new(),
    })
} 

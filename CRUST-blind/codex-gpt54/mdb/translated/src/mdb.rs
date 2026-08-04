use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const DB_NAME_MAX: usize = 128;
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
pub struct MdbStatus {
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

fn invalid_data(msg: &str) -> MdbError {
    MdbError::Io(io::Error::new(io::ErrorKind::InvalidData, msg))
}

fn invalid_input(msg: &str) -> MdbError {
    MdbError::Io(io::Error::new(io::ErrorKind::InvalidInput, msg))
}

fn path_with_suffix<P: AsRef<Path>>(path: P, suffix: &str) -> PathBuf {
    let path = path.as_ref();
    let mut rendered = path.as_os_str().to_os_string();
    rendered.push(suffix);
    PathBuf::from(rendered)
}

fn read_u32(file: &mut File) -> io::Result<u32> {
    let mut buf = [0u8; MDB_PTR_SIZE];
    file.read_exact(&mut buf)?;
    Ok(u32::from_ne_bytes(buf))
}

fn write_u32(file: &mut File, value: u32) -> io::Result<()> {
    file.write_all(&value.to_ne_bytes())
}

impl Mdb {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let super_path = path_with_suffix(&path, ".db.super");
        let index_path = path_with_suffix(&path, ".db.index");
        let data_path = path_with_suffix(path, ".db.data");

        let mut fp_superblock = File::open(super_path)?;
        let mut superblock = String::new();
        fp_superblock.read_to_string(&mut superblock)?;

        let mut fields = superblock.split_whitespace();
        let db_name = fields
            .next()
            .ok_or_else(|| invalid_data("missing db_name in superblock"))?
            .to_string();
        let key_size_max = fields
            .next()
            .ok_or_else(|| invalid_data("missing key_size_max in superblock"))?
            .parse::<u16>()
            .map_err(|_| invalid_data("invalid key_size_max in superblock"))?;
        let data_size_max = fields
            .next()
            .ok_or_else(|| invalid_data("missing data_size_max in superblock"))?
            .parse::<u32>()
            .map_err(|_| invalid_data("invalid data_size_max in superblock"))?;
        let hash_buckets = fields
            .next()
            .ok_or_else(|| invalid_data("missing hash_buckets in superblock"))?
            .parse::<u32>()
            .map_err(|_| invalid_data("invalid hash_buckets in superblock"))?;
        let items_max = fields
            .next()
            .ok_or_else(|| invalid_data("missing items_max in superblock"))?
            .parse::<u32>()
            .map_err(|_| invalid_data("invalid items_max in superblock"))?;

        let options = MdbOptions {
            db_name: db_name.clone(),
            key_size_max,
            data_size_max,
            hash_buckets,
            items_max,
        };
        let index_record_size =
            u32::from(key_size_max) + (MDB_PTR_SIZE as u32) * 2 + MDB_DATALEN_SIZE as u32;

        let fp_index = OpenOptions::new().read(true).write(true).open(index_path)?;
        let fp_data = OpenOptions::new().read(true).write(true).open(data_path)?;

        Ok(Self {
            db_name,
            fp_superblock,
            fp_index,
            fp_data,
            options,
            index_record_size,
        })
    }

    pub fn create<P: AsRef<Path>>(path: P, options: MdbOptions) -> Result<Self> {
        if options.db_name.len() > DB_NAME_MAX {
            return Err(invalid_input("db_name too large"));
        }

        let super_path = path_with_suffix(&path, ".db.super");
        let index_path = path_with_suffix(&path, ".db.index");
        let data_path = path_with_suffix(path, ".db.data");

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

        let fp_data = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .read(true)
            .open(data_path)?;

        let index_record_size = u32::from(options.key_size_max)
            + (MDB_PTR_SIZE as u32) * 2
            + MDB_DATALEN_SIZE as u32;

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
        let key_bytes = key.as_bytes();
        if key_bytes.len() > usize::from(self.options.key_size_max) {
            return Err(MdbError::KeySizeTooLarge);
        }

        let value_bytes = value.as_bytes();
        let value_size =
            u32::try_from(value_bytes.len()).map_err(|_| MdbError::ValueSizeTooLarge)?;
        if value_size > self.options.data_size_max {
            return Err(MdbError::ValueSizeTooLarge);
        }

        let bucket = self.hash(key) % self.options.hash_buckets;
        let mut save_ptr = MDB_PTR_SIZE as u32 * (bucket + 1);
        let mut ptr = self.read_bucket(bucket)?;

        while ptr != 0 {
            let index = self.read_index(ptr)?;
            if index.key == key_bytes {
                self.data_free(index.value_ptr, index.value_size)?;
                let mut value_ptr = 0;
                self.data_alloc(value_size, &mut value_ptr)?;
                self.write_data(value_ptr, value_bytes, value_size)?;
                self.write_index(ptr, key_bytes, value_ptr, value_size)?;
                return Ok(());
            }
            save_ptr = ptr;
            ptr = index.next_ptr;
        }

        let mut index_ptr = 0;
        self.index_alloc(&mut index_ptr)?;

        let mut value_ptr = 0;
        if let Err(err) = self.data_alloc(value_size, &mut value_ptr) {
            let _ = self.index_free(index_ptr);
            return Err(err);
        }

        if let Err(err) = self.write_data(value_ptr, value_bytes, value_size) {
            let _ = self.data_free(value_ptr, value_size);
            let _ = self.index_free(index_ptr);
            return Err(err);
        }

        if let Err(err) = self.write_index(index_ptr, key_bytes, value_ptr, value_size) {
            let _ = self.data_free(value_ptr, value_size);
            let _ = self.index_free(index_ptr);
            return Err(err);
        }

        if let Err(err) = self.write_nextptr(save_ptr, index_ptr) {
            let _ = self.data_free(value_ptr, value_size);
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

    fn read_bucket(&mut self, bucket: u32) -> Result<MdbPtr> {
        self.fp_index
            .seek(SeekFrom::Start(u64::from(MDB_PTR_SIZE as u32 * (bucket + 1))))?;
        Ok(read_u32(&mut self.fp_index)?)
    }

    fn read_index(&mut self, idxptr: MdbPtr) -> Result<MdbIndex> {
        self.fp_index.seek(SeekFrom::Start(u64::from(idxptr)))?;

        let next_ptr = read_u32(&mut self.fp_index)?;
        let mut key_buf = vec![0u8; usize::from(self.options.key_size_max)];
        self.fp_index.read_exact(&mut key_buf)?;
        let value_ptr = read_u32(&mut self.fp_index)?;
        let value_size = read_u32(&mut self.fp_index)?;

        let key_len = key_buf
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(key_buf.len());
        key_buf.truncate(key_len);

        Ok(MdbIndex {
            next_ptr,
            value_ptr,
            value_size,
            key: key_buf,
        })
    }

    fn write_bucket(&mut self, bucket: u32, ptr: MdbPtr) -> Result<()> {
        self.fp_index
            .seek(SeekFrom::Start(u64::from(MDB_PTR_SIZE as u32 * (bucket + 1))))?;
        write_u32(&mut self.fp_index, ptr)?;
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
        self.fp_index
            .seek(SeekFrom::Start(u64::from(idxptr + MDB_PTR_SIZE as u32)))?;
        self.fp_index.write_all(key)?;

        let key_area_end = u64::from(idxptr)
            + MDB_PTR_SIZE as u64
            + u64::from(self.options.key_size_max);
        self.fp_index.seek(SeekFrom::Start(key_area_end))?;
        write_u32(&mut self.fp_index, value_ptr)?;
        write_u32(&mut self.fp_index, value_size)?;
        self.fp_index.flush()?;
        Ok(())
    }

    fn read_nextptr(&mut self, idxptr: MdbPtr) -> Result<MdbPtr> {
        self.fp_index.seek(SeekFrom::Start(u64::from(idxptr)))?;
        Ok(read_u32(&mut self.fp_index)?)
    }

    fn write_nextptr(&mut self, ptr: MdbPtr, nextptr: MdbPtr) -> Result<()> {
        self.fp_index.seek(SeekFrom::Start(u64::from(ptr)))?;
        write_u32(&mut self.fp_index, nextptr)?;
        self.fp_index.flush()?;
        Ok(())
    }

    fn read_data(&mut self, valptr: MdbPtr, valsize: MdbSize, buf: &mut [u8]) -> Result<usize> {
        let valsize = usize::try_from(valsize).map_err(|_| MdbError::AllocationFailed)?;
        if buf.len() < valsize + 1 {
            return Err(MdbError::BufferSizeTooSmall);
        }

        self.fp_data.seek(SeekFrom::Start(u64::from(valptr)))?;
        self.fp_data.read_exact(&mut buf[..valsize])?;
        buf[valsize] = 0;
        Ok(valsize)
    }

    fn write_data(&mut self, valptr: MdbPtr, value: &[u8], valsize: MdbSize) -> Result<()> {
        let valsize = usize::try_from(valsize).map_err(|_| MdbError::AllocationFailed)?;
        self.fp_data.seek(SeekFrom::Start(u64::from(valptr)))?;
        self.fp_data.write_all(&value[..valsize])?;
        self.fp_data.flush()?;
        Ok(())
    }

    fn stretch_index_file(&mut self, ptr: &mut MdbPtr) -> Result<()> {
        let end = self.fp_index.seek(SeekFrom::End(0))?;
        *ptr = u32::try_from(end).map_err(|_| MdbError::AllocationFailed)?;

        let zeros = vec![
            0u8;
            usize::try_from(self.index_record_size).map_err(|_| MdbError::AllocationFailed)?
        ];
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
            return Ok(());
        }

        self.stretch_index_file(ptr)
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
                match self.fp_data.read(&mut byte)? {
                    0 => break,
                    _ => {}
                }
            }

            let start_ptr = self.fp_data.stream_position()?;
            while byte[0] == 0 {
                match self.fp_data.read(&mut byte)? {
                    0 => break,
                    _ => {}
                }
            }
            let end_ptr = self.fp_data.stream_position()?;

            if end_ptr - start_ptr >= u64::from(size) + 2 {
                *ptr = u32::try_from(start_ptr + 1).map_err(|_| MdbError::AllocationFailed)?;
                return Ok(());
            }

            if byte[0] == 0 && self.fp_data.stream_position()? == end_ptr {
                break;
            }
        }

        let end_ptr = self.fp_data.seek(SeekFrom::End(0))?;
        let zeros = vec![0u8; usize::try_from(size).map_err(|_| MdbError::AllocationFailed)?];
        self.fp_data.write_all(&zeros)?;
        self.fp_data.flush()?;
        *ptr = u32::try_from(end_ptr).map_err(|_| MdbError::AllocationFailed)?;
        Ok(())
    }

    fn index_free(&mut self, ptr: MdbPtr) -> Result<()> {
        self.fp_index.seek(SeekFrom::Start(0))?;
        let freeptr = read_u32(&mut self.fp_index)?;

        self.fp_index.seek(SeekFrom::Start(0))?;
        write_u32(&mut self.fp_index, ptr)?;

        self.fp_index.seek(SeekFrom::Start(u64::from(ptr)))?;
        write_u32(&mut self.fp_index, freeptr)?;
        let zeros = vec![0u8; usize::from(self.options.key_size_max)];
        self.fp_index.write_all(&zeros)?;
        self.fp_index.flush()?;
        Ok(())
    }

    fn data_free(&mut self, ptr: MdbPtr, size: MdbSize) -> Result<()> {
        self.fp_data.seek(SeekFrom::Start(u64::from(ptr)))?;
        let zeros = vec![0u8; usize::try_from(size).map_err(|_| MdbError::AllocationFailed)?];
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
        let mut ret = 0u32;
        for (i, byte) in key.bytes().enumerate() {
            ret = ret.wrapping_add(u32::from(byte).wrapping_mul(i as u32));
        }
        ret
    }

    fn close(&mut self) -> Result<()> {
        self.fp_superblock.flush()?;
        self.fp_index.flush()?;
        self.fp_data.flush()?;
        Ok(())
    }
}

impl Drop for Mdb {
    fn drop(&mut self) {
        let _ = self.close();
        let _ = &self.db_name;
        let _ = Self::alloc as fn() -> Result<()>;
        let _ = Self::free as fn() -> Result<()>;
        let _ = Self::write_bucket as fn(&mut Self, u32, MdbPtr) -> Result<()>;
    }
}

pub fn mdb_status() -> Result<MdbStatus> {
    Ok(MdbStatus {
        code: MdbStatusCode::MDB_OK as u8,
        desc: "OK".to_string(),
    })
}

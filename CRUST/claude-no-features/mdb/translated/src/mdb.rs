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

fn parse_err() -> MdbError {
    MdbError::Io(io::Error::new(io::ErrorKind::InvalidData, "parse error"))
}

impl Mdb {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy().into_owned();

        let super_path = format!("{}.db.super", path_str);
        let super_file = File::open(&super_path)?;

        let mut reader = BufReader::new(super_file);

        let mut db_name_line = String::new();
        reader.read_line(&mut db_name_line)?;
        let db_name = db_name_line.trim_end_matches(|c| c == '\n' || c == '\r').to_string();

        let mut line = String::new();
        reader.read_line(&mut line)?;
        let key_size_max: u16 = line.trim().parse().map_err(|_| parse_err())?;
        line.clear();

        reader.read_line(&mut line)?;
        let data_size_max: u32 = line.trim().parse().map_err(|_| parse_err())?;
        line.clear();

        reader.read_line(&mut line)?;
        let hash_buckets: u32 = line.trim().parse().map_err(|_| parse_err())?;
        line.clear();

        reader.read_line(&mut line)?;
        let items_max: u32 = line.trim().parse().map_err(|_| parse_err())?;

        let fp_superblock = reader.into_inner();

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

        let index_path = format!("{}.db.index", path_str);
        let fp_index = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&index_path)?;

        let data_path = format!("{}.db.data", path_str);
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
        let path_str = path.as_ref().to_string_lossy().into_owned();
        let db_name = options.db_name.clone();

        let index_record_size = options.key_size_max as u32
            + (MDB_PTR_SIZE * 2) as u32
            + MDB_DATALEN_SIZE as u32;

        let super_path = format!("{}.db.super", path_str);
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

        let index_path = format!("{}.db.index", path_str);
        let mut fp_index = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&index_path)?;

        let zero_ptr: MdbPtr = 0;
        fp_index.write_all(&zero_ptr.to_le_bytes())?;
        for _ in 0..options.hash_buckets {
            fp_index.write_all(&zero_ptr.to_le_bytes())?;
        }
        fp_index.flush()?;

        let data_path = format!("{}.db.data", path_str);
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
        let key_bytes = key.as_bytes();
        let bucket = self.hash(key) % self.options.hash_buckets;
        let mut ptr = self.read_bucket(bucket)?;

        while ptr != 0 {
            let index = self.read_index(ptr)?;
            if index.key.as_slice() == key_bytes {
                return self.read_data(index.value_ptr, index.value_size, buf);
            }
            ptr = index.next_ptr;
        }

        Err(MdbError::KeyNotFound)
    }

    pub fn write(&mut self, key: &str, value: &str) -> Result<()> {
        let key_bytes = key.as_bytes();
        let value_bytes = value.as_bytes();

        let bucket = self.hash(key) % self.options.hash_buckets;

        if key_bytes.len() > self.options.key_size_max as usize {
            return Err(MdbError::KeySizeTooLarge);
        }
        if value_bytes.len() > self.options.data_size_max as usize {
            return Err(MdbError::ValueSizeTooLarge);
        }
        let value_size = value_bytes.len() as MdbSize;

        let mut save_ptr: MdbPtr = (MDB_PTR_SIZE as MdbPtr) * (bucket + 1);
        let mut ptr = self.read_bucket(bucket)?;

        let mut existing: Option<MdbIndex> = None;
        while ptr != 0 {
            let index = self.read_index(ptr)?;
            if index.key.as_slice() == key_bytes {
                existing = Some(index);
                break;
            }
            save_ptr = ptr;
            ptr = index.next_ptr;
        }

        if let Some(idx) = existing {
            // update existing record
            self.data_free(idx.value_ptr, idx.value_size)?;
            let mut value_ptr: MdbPtr = 0;
            self.data_alloc(value_size, &mut value_ptr)?;
            self.write_data(value_ptr, value_bytes, value_size)?;
            self.write_index(ptr, key_bytes, value_ptr, value_size)?;
            Ok(())
        } else {
            let mut index_ptr: MdbPtr = 0;
            self.index_alloc(&mut index_ptr)?;
            let mut value_ptr: MdbPtr = 0;
            self.data_alloc(value_size, &mut value_ptr)?;
            self.write_data(value_ptr, value_bytes, value_size)?;
            self.write_index(index_ptr, key_bytes, value_ptr, value_size)?;
            self.write_nextptr(save_ptr, index_ptr)?;
            Ok(())
        }
    }

    pub fn delete(&mut self, key: &str) -> Result<()> {
        let key_bytes = key.as_bytes();
        let bucket = self.hash(key) % self.options.hash_buckets;

        let mut save_ptr: MdbPtr = (MDB_PTR_SIZE as MdbPtr) * (bucket + 1);
        let mut ptr = self.read_bucket(bucket)?;

        let mut found: Option<MdbIndex> = None;
        while ptr != 0 {
            let index = self.read_index(ptr)?;
            if index.key.as_slice() == key_bytes {
                found = Some(index);
                break;
            }
            save_ptr = ptr;
            ptr = index.next_ptr;
        }

        match found {
            None => Err(MdbError::KeyNotFound),
            Some(idx) => {
                self.data_free(idx.value_ptr, idx.value_size)?;
                self.index_free(ptr)?;
                self.write_nextptr(save_ptr, idx.next_ptr)?;
                Ok(())
            }
        }
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

        let mut next_ptr_buf = [0u8; MDB_PTR_SIZE];
        self.fp_index.read_exact(&mut next_ptr_buf)?;
        let next_ptr = MdbPtr::from_le_bytes(next_ptr_buf);

        let key_size_max = self.options.key_size_max as usize;
        let mut key_buf = vec![0u8; key_size_max];
        self.fp_index.read_exact(&mut key_buf)?;
        let key_end = key_buf.iter().position(|&b| b == 0).unwrap_or(key_size_max);
        key_buf.truncate(key_end);

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
        let offset = (MDB_PTR_SIZE as u64) * ((bucket as u64) + 1);
        self.fp_index.seek(SeekFrom::Start(offset))?;
        self.fp_index.write_all(&ptr.to_le_bytes())?;
        self.fp_index.flush()?;
        Ok(())
    }

    fn write_index(&mut self, idxptr: MdbPtr, key: &[u8], value_ptr: MdbPtr, value_size: MdbSize) -> Result<()> {
        let key_pos = (idxptr as u64) + (MDB_PTR_SIZE as u64);
        self.fp_index.seek(SeekFrom::Start(key_pos))?;
        self.fp_index.write_all(key)?;

        let value_ptr_pos = key_pos + (self.options.key_size_max as u64);
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
        if (buf.len() as u64) < (valsize as u64 + 1) {
            return Err(MdbError::BufferSizeTooSmall);
        }
        self.fp_data.seek(SeekFrom::Start(valptr as u64))?;
        let target = &mut buf[..valsize as usize];
        self.fp_data.read_exact(target)?;
        Ok(valsize as usize)
    }

    fn write_data(&mut self, valptr: MdbPtr, value: &[u8], valsize: MdbSize) -> Result<()> {
        self.fp_data.seek(SeekFrom::Start(valptr as u64))?;
        self.fp_data.write_all(&value[..valsize as usize])?;
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

        loop {
            let mut buf = [0u8; 1];
            let n = self.fp_data.read(&mut buf)?;
            if n == 0 {
                break;
            }
            let mut byte = buf[0];
            let mut feof = false;

            // skip non-zero bytes
            while byte != 0 {
                let n = self.fp_data.read(&mut buf)?;
                if n == 0 {
                    feof = true;
                    break;
                }
                byte = buf[0];
            }

            let start_ptr = self.fp_data.stream_position()? as MdbPtr;

            // skip zero bytes
            while !feof && byte == 0 {
                let n = self.fp_data.read(&mut buf)?;
                if n == 0 {
                    feof = true;
                    break;
                }
                byte = buf[0];
            }
            let end_ptr = self.fp_data.stream_position()? as MdbPtr;

            if end_ptr.saturating_sub(start_ptr) >= size + 2 {
                *ptr = start_ptr + 1;
                return Ok(());
            }

            if feof {
                break;
            }
        }

        let end_ptr = self.fp_data.seek(SeekFrom::End(0))? as MdbPtr;
        let zeros = vec![0u8; size as usize];
        self.fp_data.write_all(&zeros)?;
        self.fp_data.flush()?;
        *ptr = end_ptr;
        Ok(())
    }

    fn index_free(&mut self, ptr: MdbPtr) -> Result<()> {
        // Read current free_ptr (at offset 0)
        self.fp_index.seek(SeekFrom::Start(0))?;
        let mut buf = [0u8; MDB_PTR_SIZE];
        self.fp_index.read_exact(&mut buf)?;
        let freeptr = MdbPtr::from_le_bytes(buf);

        // Write `ptr` as new free_ptr at offset 0
        self.fp_index.seek(SeekFrom::Start(0))?;
        self.fp_index.write_all(&ptr.to_le_bytes())?;

        // At the freed entry, write old freeptr as its next_ptr, and clear the key
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

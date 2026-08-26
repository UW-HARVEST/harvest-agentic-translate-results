#![allow(dead_code)]

use libloading::Library;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::ptr;

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;
pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Debug)]
pub struct StringBlock {
    pub next: *mut StringBlock,
    pub storage: [c_char; 8],
}

#[repr(C)]
#[derive(Debug)]
pub struct StringArena {
    pub storage: *mut StringBlock,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

impl Default for StringArena {
    fn default() -> Self {
        Self {
            storage: ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct HashBucket {
    pub hash: [usize; 8],
    pub index: [isize; 8],
}

#[repr(C)]
#[derive(Debug)]
pub struct HashIndex {
    pub temp_key: *mut c_char,
    pub slot_count: usize,
    pub used_count: usize,
    pub used_count_threshold: usize,
    pub used_count_shrink_threshold: usize,
    pub tombstone_count: usize,
    pub tombstone_count_threshold: usize,
    pub seed: usize,
    pub slot_count_log2: usize,
    pub string: StringArena,
    pub storage: *mut HashBucket,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BinEntry {
    pub key: u64,
    pub value: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct StringEntry {
    pub key: *mut c_char,
    pub value: i64,
}

type ArrGrow = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type ArrFree = unsafe extern "C" fn(*mut c_void);
type RandSeed = unsafe extern "C" fn(usize);
type HashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
type HashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
type HmFree = unsafe extern "C" fn(*mut c_void, usize);
type HmGetTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
type HmGet = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type HmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type HmPut = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type ShMode = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
type HmDel =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
type StrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
type StrReset = unsafe extern "C" fn(*mut StringArena);
type StrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
type ShPuts = unsafe extern "C" fn(c_int);

pub struct Api {
    _library: Library,
    pub arrgrow: ArrGrow,
    pub arrfree: ArrFree,
    pub rand_seed: RandSeed,
    pub hash_string: HashString,
    pub hash_bytes: HashBytes,
    pub hmfree: HmFree,
    pub hmget_ts: HmGetTs,
    pub hmget: HmGet,
    pub hmput_default: HmPutDefault,
    pub hmput: HmPut,
    pub shmode: ShMode,
    pub hmdel: HmDel,
    pub stralloc: StrAlloc,
    pub strreset: StrReset,
    pub strkey: StrKey,
    pub sh_puts: ShPuts,
}

impl Api {
    pub unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        macro_rules! function {
            ($name:literal, $ty:ty) => {
                *unsafe { library.get::<$ty>(concat!($name, "\0").as_bytes()) }
                    .unwrap_or_else(|error| panic!("missing {}: {error}", $name))
            };
        }
        Self {
            arrgrow: function!("stbds_arrgrowf", ArrGrow),
            arrfree: function!("stbds_arrfreef", ArrFree),
            rand_seed: function!("stbds_rand_seed", RandSeed),
            hash_string: function!("stbds_hash_string", HashString),
            hash_bytes: function!("stbds_hash_bytes", HashBytes),
            hmfree: function!("stbds_hmfree_func", HmFree),
            hmget_ts: function!("stbds_hmget_key_ts", HmGetTs),
            hmget: function!("stbds_hmget_key", HmGet),
            hmput_default: function!("stbds_hmput_default", HmPutDefault),
            hmput: function!("stbds_hmput_key", HmPut),
            shmode: function!("stbds_shmode_func", ShMode),
            hmdel: function!("stbds_hmdel_key", HmDel),
            stralloc: function!("stbds_stralloc", StrAlloc),
            strreset: function!("stbds_strreset", StrReset),
            strkey: function!("strkey", StrKey),
            sh_puts: function!("sh_puts", ShPuts),
            _library: library,
        }
    }
}

pub fn library_paths() -> (PathBuf, PathBuf) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        root.join("c_src/build/libtranslated_rust.so"),
        root.join("target/release/libsh_puts_lib.so"),
    )
}

pub unsafe fn header(data: *mut c_void) -> *mut ArrayHeader {
    unsafe { data.cast::<ArrayHeader>().sub(1) }
}

pub unsafe fn raw_map(map: *mut c_void, elem_size: usize) -> *mut c_void {
    unsafe { map.cast::<u8>().sub(elem_size).cast() }
}

pub unsafe fn map_header(map: *mut c_void, elem_size: usize) -> *mut ArrayHeader {
    unsafe { header(raw_map(map, elem_size)) }
}

pub unsafe fn table(map: *mut c_void, elem_size: usize) -> *mut HashIndex {
    unsafe { (*map_header(map, elem_size)).hash_table.cast() }
}

#[derive(Debug, PartialEq, Eq)]
pub struct TableSnapshot {
    pub slot_count: usize,
    pub used_count: usize,
    pub used_count_threshold: usize,
    pub used_count_shrink_threshold: usize,
    pub tombstone_count: usize,
    pub tombstone_count_threshold: usize,
    pub seed: usize,
    pub slot_count_log2: usize,
    pub string_remaining: usize,
    pub string_block: u8,
    pub string_mode: u8,
    pub buckets: Vec<([usize; 8], [isize; 8])>,
}

pub unsafe fn table_snapshot(map: *mut c_void, elem_size: usize) -> Option<TableSnapshot> {
    let table = unsafe { table(map, elem_size) };
    if table.is_null() {
        return None;
    }
    let mut buckets = Vec::new();
    for index in 0..unsafe { (*table).slot_count } / 8 {
        let bucket = unsafe { *(*table).storage.add(index) };
        buckets.push((bucket.hash, bucket.index));
    }
    Some(TableSnapshot {
        slot_count: unsafe { (*table).slot_count },
        used_count: unsafe { (*table).used_count },
        used_count_threshold: unsafe { (*table).used_count_threshold },
        used_count_shrink_threshold: unsafe { (*table).used_count_shrink_threshold },
        tombstone_count: unsafe { (*table).tombstone_count },
        tombstone_count_threshold: unsafe { (*table).tombstone_count_threshold },
        seed: unsafe { (*table).seed },
        slot_count_log2: unsafe { (*table).slot_count_log2 },
        string_remaining: unsafe { (*table).string.remaining },
        string_block: unsafe { (*table).string.block },
        string_mode: unsafe { (*table).string.mode },
        buckets,
    })
}

#[derive(Debug, PartialEq, Eq)]
pub struct BinMapSnapshot {
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub entries: Vec<BinEntry>,
    pub table: Option<TableSnapshot>,
}

pub unsafe fn bin_map_snapshot(map: *mut c_void) -> BinMapSnapshot {
    let elem_size = size_of::<BinEntry>();
    let header = unsafe { &*map_header(map, elem_size) };
    let count = header.length.saturating_sub(1);
    let entries = unsafe { std::slice::from_raw_parts(map.cast::<BinEntry>(), count) }.to_vec();
    BinMapSnapshot {
        length: header.length,
        capacity: header.capacity,
        temp: header.temp,
        entries,
        table: unsafe { table_snapshot(map, elem_size) },
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct StringMapSnapshot {
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub entries: Vec<(Vec<u8>, i64)>,
    pub table: Option<TableSnapshot>,
}

pub unsafe fn string_map_snapshot(map: *mut c_void) -> StringMapSnapshot {
    let elem_size = size_of::<StringEntry>();
    let header = unsafe { &*map_header(map, elem_size) };
    let count = header.length.saturating_sub(1);
    let mut entries = Vec::with_capacity(count);
    for index in 0..count {
        let entry = unsafe { &*map.cast::<StringEntry>().add(index) };
        entries.push((
            unsafe { CStr::from_ptr(entry.key) }.to_bytes().to_vec(),
            entry.value,
        ));
    }
    StringMapSnapshot {
        length: header.length,
        capacity: header.capacity,
        temp: header.temp,
        entries,
        table: unsafe { table_snapshot(map, elem_size) },
    }
}

pub unsafe fn put_bin(api: &Api, map: *mut c_void, key: u64, value: i64) -> *mut c_void {
    let mut key = key;
    let map = unsafe {
        (api.hmput)(
            map,
            size_of::<BinEntry>(),
            ptr::addr_of_mut!(key).cast(),
            size_of::<u64>(),
            HM_BINARY,
        )
    };
    let index = unsafe { (*map_header(map, size_of::<BinEntry>())).temp as usize };
    unsafe { (*map.cast::<BinEntry>().add(index)).value = value };
    map
}

pub unsafe fn put_string(
    api: &Api,
    map: *mut c_void,
    key: *mut c_char,
    value: i64,
    mode: c_int,
) -> *mut c_void {
    let map = unsafe {
        (api.hmput)(
            map,
            size_of::<StringEntry>(),
            key.cast(),
            size_of::<*mut c_char>(),
            mode,
        )
    };
    let index = unsafe { (*map_header(map, size_of::<StringEntry>())).temp as usize };
    unsafe { (*map.cast::<StringEntry>().add(index)).value = value };
    map
}

pub unsafe fn free_bin_map(api: &Api, map: *mut c_void) {
    if !map.is_null() {
        unsafe { (api.hmfree)(raw_map(map, size_of::<BinEntry>()), size_of::<BinEntry>()) };
    }
}

pub unsafe fn free_string_map(api: &Api, map: *mut c_void) {
    if !map.is_null() {
        unsafe {
            (api.hmfree)(
                raw_map(map, size_of::<StringEntry>()),
                size_of::<StringEntry>(),
            )
        };
    }
}

unsafe extern "C" {
    fn pipe(fds: *mut c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

pub unsafe fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let mut fds = [0; 2];
    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);
    assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0);
    let saved = unsafe { dup(1) };
    assert!(saved >= 0);
    assert_eq!(unsafe { dup2(fds[1], 1) }, 1);
    assert_eq!(unsafe { close(fds[1]) }, 0);
    call();
    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);
    assert_eq!(unsafe { dup2(saved, 1) }, 1);
    assert_eq!(unsafe { close(saved) }, 0);
    let mut output = Vec::new();
    let mut reader = unsafe { File::from_raw_fd(fds[0]) };
    reader.read_to_end(&mut output).unwrap();
    output
}

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    pub fn fill(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            *byte = self.next_u64() as u8;
        }
    }
}

#![allow(dead_code)]
#![allow(unsafe_op_in_unsafe_fn)]

use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::path::PathBuf;
use std::ptr;
use std::sync::Mutex;

pub static TEST_LOCK: Mutex<()> = Mutex::new(());

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;
pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

pub type ArrGrow = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type ArrFree = unsafe extern "C" fn(*mut c_void);
pub type RandSeed = unsafe extern "C" fn(usize);
pub type HashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type HashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type HmFree = unsafe extern "C" fn(*mut c_void, usize);
pub type HmGet = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type HmGetTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type HmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type HmPut = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type HmDel =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type ShMode = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type StrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
pub type StrReset = unsafe extern "C" fn(*mut StringArena);
pub type StrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type ShPuts = unsafe extern "C" fn(c_int);

pub struct Api {
    _library: Library,
    pub arrgrow: ArrGrow,
    pub arrfree: ArrFree,
    pub rand_seed: RandSeed,
    pub hash_bytes: HashBytes,
    pub hash_string: HashString,
    pub hmfree: HmFree,
    pub hmget: HmGet,
    pub hmget_ts: HmGetTs,
    pub hmput_default: HmPutDefault,
    pub hmput: HmPut,
    pub hmdel: HmDel,
    pub shmode: ShMode,
    pub stralloc: StrAlloc,
    pub strreset: StrReset,
    pub strkey: StrKey,
    pub sh_puts: ShPuts,
}

unsafe fn load_symbol<T: Copy>(library: &Library, name: &[u8]) -> T {
    *library.get::<T>(name).unwrap()
}

impl Api {
    pub unsafe fn load(path: PathBuf) -> Self {
        let library = Library::new(path).unwrap();
        Self {
            arrgrow: load_symbol(&library, b"stbds_arrgrowf\0"),
            arrfree: load_symbol(&library, b"stbds_arrfreef\0"),
            rand_seed: load_symbol(&library, b"stbds_rand_seed\0"),
            hash_bytes: load_symbol(&library, b"stbds_hash_bytes\0"),
            hash_string: load_symbol(&library, b"stbds_hash_string\0"),
            hmfree: load_symbol(&library, b"stbds_hmfree_func\0"),
            hmget: load_symbol(&library, b"stbds_hmget_key\0"),
            hmget_ts: load_symbol(&library, b"stbds_hmget_key_ts\0"),
            hmput_default: load_symbol(&library, b"stbds_hmput_default\0"),
            hmput: load_symbol(&library, b"stbds_hmput_key\0"),
            hmdel: load_symbol(&library, b"stbds_hmdel_key\0"),
            shmode: load_symbol(&library, b"stbds_shmode_func\0"),
            stralloc: load_symbol(&library, b"stbds_stralloc\0"),
            strreset: load_symbol(&library, b"stbds_strreset\0"),
            strkey: load_symbol(&library, b"strkey\0"),
            sh_puts: load_symbol(&library, b"sh_puts\0"),
            _library: library,
        }
    }
}

pub unsafe fn load_pair() -> (Api, Api) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_path = manifest
        .parent()
        .unwrap()
        .join("c_src/build/libharvest-work-i1tAuE.so");
    let rust_path = std::env::var_os("RUST_TRANSLATION_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest.join("target/release/libsh_puts_lib.so"));
    (Api::load(c_path), Api::load(rust_path))
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
pub struct StringBlock {
    pub next: *mut StringBlock,
    pub storage: [c_char; 8],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
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
pub struct HashBucket {
    pub hash: [usize; 8],
    pub index: [isize; 8],
}

#[repr(C)]
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

pub unsafe fn header(array: *mut c_void) -> *mut ArrayHeader {
    array
        .cast::<u8>()
        .sub(size_of::<ArrayHeader>())
        .cast::<ArrayHeader>()
}

pub unsafe fn raw_from_map(map: *mut c_void, element_size: usize) -> *mut c_void {
    map.cast::<u8>().sub(element_size).cast()
}

pub unsafe fn map_from_raw(array: *mut c_void, element_size: usize) -> *mut c_void {
    array.cast::<u8>().add(element_size).cast()
}

pub unsafe fn map_header(map: *mut c_void, element_size: usize) -> *mut ArrayHeader {
    header(raw_from_map(map, element_size))
}

pub unsafe fn table(map: *mut c_void, element_size: usize) -> *mut HashIndex {
    (*map_header(map, element_size)).hash_table.cast()
}

pub unsafe fn free_map(api: &Api, map: *mut c_void, element_size: usize) {
    if !map.is_null() {
        (api.hmfree)(raw_from_map(map, element_size), element_size);
    }
}

pub unsafe fn block_count(arena: &StringArena) -> usize {
    let mut count = 0;
    let mut block = arena.storage;
    while !block.is_null() {
        count += 1;
        block = (*block).next;
    }
    count
}

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 ^ (self.0 >> 29)
    }

    pub fn next_usize(&mut self) -> usize {
        self.next_u64() as usize
    }

    pub fn fill(&mut self, bytes: &mut [u8]) {
        for chunk in bytes.chunks_mut(8) {
            let random = self.next_u64().to_le_bytes();
            chunk.copy_from_slice(&random[..chunk.len()]);
        }
    }
}

unsafe extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
    fn fflush(stream: *mut c_void) -> c_int;
    fn _exit(status: c_int) -> !;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChildOutcome {
    pub signal: c_int,
    pub exit_code: c_int,
}

fn decode_wait_status(status: c_int) -> ChildOutcome {
    let signal = status & 0x7f;
    let exit_code = if signal == 0 {
        (status >> 8) & 0xff
    } else {
        -1
    };
    ChildOutcome { signal, exit_code }
}

pub unsafe fn isolated<F: FnOnce()>(operation: F) -> ChildOutcome {
    fflush(ptr::null_mut());
    let pid = fork();
    assert!(pid >= 0);
    if pid == 0 {
        operation();
        fflush(ptr::null_mut());
        _exit(0);
    }
    let mut status = 0;
    assert_eq!(waitpid(pid, &mut status, 0), pid);
    decode_wait_status(status)
}

pub unsafe fn capture_stdout<F: FnOnce()>(operation: F) -> (Vec<u8>, ChildOutcome) {
    fflush(ptr::null_mut());
    let mut fds = [-1, -1];
    assert_eq!(pipe(fds.as_mut_ptr()), 0);
    let pid = fork();
    assert!(pid >= 0);
    if pid == 0 {
        close(fds[0]);
        assert_eq!(dup2(fds[1], 1), 1);
        close(fds[1]);
        operation();
        fflush(ptr::null_mut());
        _exit(0);
    }

    close(fds[1]);
    let mut output = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let count = read(fds[0], buffer.as_mut_ptr().cast(), buffer.len());
        if count == 0 {
            break;
        }
        assert!(count > 0);
        output.extend_from_slice(&buffer[..count as usize]);
    }
    close(fds[0]);
    let mut status = 0;
    assert_eq!(waitpid(pid, &mut status, 0), pid);
    (output, decode_wait_status(status))
}

pub unsafe fn duplicate_fd(fd: c_int) -> c_int {
    dup(fd)
}

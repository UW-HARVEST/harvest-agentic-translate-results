//! Shared differential-testing harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and driven
//! exclusively through their exported C symbols.  Nothing in the Rust crate is
//! called directly, so the `#[no_mangle] extern "C"` wrappers are under test
//! too.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// Layout mirrors of the C structs (x86-64 SysV)
// ---------------------------------------------------------------------------

pub const HEADER_SIZE: usize = 32; // {size_t,size_t,void*,ptrdiff_t}
pub const BUCKET_LENGTH: usize = 8;
pub const BUCKET_SHIFT: usize = 3;
pub const BUCKET_MASK: usize = 7;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StringArena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

impl StringArena {
    pub fn zeroed() -> Self {
        StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct HashBucket {
    pub hash: [usize; BUCKET_LENGTH],
    pub index: [isize; BUCKET_LENGTH],
}

#[repr(C)]
#[derive(Copy, Clone)]
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

pub const STBDS_HM_BINARY: c_int = 0;
pub const STBDS_HM_STRING: c_int = 1;

pub const STBDS_SH_NONE: c_int = 0;
pub const STBDS_SH_DEFAULT: c_int = 1;
pub const STBDS_SH_STRDUP: c_int = 2;
pub const STBDS_SH_ARENA: c_int = 3;

// Sanity: the mirrors must match what the C compiler produced.
const _: () = assert!(std::mem::size_of::<ArrayHeader>() == 32);
const _: () = assert!(std::mem::size_of::<StringArena>() == 24);
const _: () = assert!(std::mem::size_of::<HashBucket>() == 128);
const _: () = assert!(std::mem::size_of::<HashIndex>() == 104);

// ---------------------------------------------------------------------------
// libc bits the harness itself needs
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn lseek(fd: c_int, off: i64, whence: c_int) -> i64;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
}

// ---------------------------------------------------------------------------
// The loaded API
// ---------------------------------------------------------------------------

pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnArrGrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrFreef = unsafe extern "C" fn(*mut c_void);
pub type FnHmFree = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmGetKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmGetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type FnHmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmPutKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnShModeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnStrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
pub type FnStrReset = unsafe extern "C" fn(*mut StringArena);
pub type FnStrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnShGeti = unsafe extern "C" fn(c_int);

pub struct Api {
    pub name: &'static str,
    pub rand_seed: FnRandSeed,
    pub hash_bytes: FnHashBytes,
    pub hash_string: FnHashString,
    pub arrgrowf: FnArrGrowf,
    pub arrfreef: FnArrFreef,
    pub hmfree_func: FnHmFree,
    pub hmget_key: FnHmGetKey,
    pub hmget_key_ts: FnHmGetKeyTs,
    pub hmput_default: FnHmPutDefault,
    pub hmput_key: FnHmPutKey,
    pub hmdel_key: FnHmDelKey,
    pub shmode_func: FnShModeFunc,
    pub stralloc: FnStrAlloc,
    pub strreset: FnStrReset,
    pub strkey: FnStrKey,
    pub sh_geti: FnShGeti,
}

unsafe fn sym<T: Copy>(lib: &'static Library, n: &[u8]) -> T {
    let s: Symbol<T> = lib
        .get(n)
        .unwrap_or_else(|e| panic!("symbol {} missing: {e}", String::from_utf8_lossy(n)));
    *s
}

unsafe fn load(name: &'static str, path: &PathBuf) -> Api {
    let lib: &'static Library = Box::leak(Box::new(
        Library::new(path).unwrap_or_else(|e| panic!("cannot dlopen {}: {e}", path.display())),
    ));
    Api {
        name,
        rand_seed: sym(lib, b"stbds_rand_seed"),
        hash_bytes: sym(lib, b"stbds_hash_bytes"),
        hash_string: sym(lib, b"stbds_hash_string"),
        arrgrowf: sym(lib, b"stbds_arrgrowf"),
        arrfreef: sym(lib, b"stbds_arrfreef"),
        hmfree_func: sym(lib, b"stbds_hmfree_func"),
        hmget_key: sym(lib, b"stbds_hmget_key"),
        hmget_key_ts: sym(lib, b"stbds_hmget_key_ts"),
        hmput_default: sym(lib, b"stbds_hmput_default"),
        hmput_key: sym(lib, b"stbds_hmput_key"),
        hmdel_key: sym(lib, b"stbds_hmdel_key"),
        shmode_func: sym(lib, b"stbds_shmode_func"),
        stralloc: sym(lib, b"stbds_stralloc"),
        strreset: sym(lib, b"stbds_strreset"),
        strkey: sym(lib, b"strkey"),
        sh_geti: sym(lib, b"sh_geti"),
    }
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = root().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                found.push(p);
            }
        }
    }
    found.sort();
    found
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("no .so found in {} - build the C library first", build.display()))
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    for prof in ["release", "debug"] {
        let p = root().join("target").join(prof).join("libsh_geti_lib.so");
        if p.exists() {
            return p;
        }
        }
    panic!("libsh_geti_lib.so not found - run `cargo build --release` first");
}

pub struct Pair {
    pub c: Api,
    pub r: Api,
}

static PAIR: OnceLock<Pair> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());

/// Acquire the global serialization lock and the loaded library pair.
///
/// Both shared objects carry mutable process-global state (`stbds_hash_seed`,
/// the `strkey` static buffer), so all differential tests must run one at a
/// time even though cargo's harness is multi-threaded.
pub fn libs() -> (MutexGuard<'static, ()>, &'static Pair) {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let p = PAIR.get_or_init(|| unsafe {
        Pair { c: load("C", &c_so_path()), r: load("Rust", &rust_so_path()) }
    });
    (g, p)
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*) so every test is reproducible
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| (self.next_u64() >> 24) as u8).collect()
    }
    /// Random NUL-free byte string of exactly `n` bytes (full 1..=255 range).
    pub fn cstring(&mut self, n: usize) -> CString {
        let v: Vec<u8> = (0..n)
            .map(|_| {
                let b = (self.next_u64() >> 24) as u8;
                if b == 0 {
                    1
                } else {
                    b
                }
            })
            .collect();
        CString::new(v).unwrap()
    }
}

// ---------------------------------------------------------------------------
// Structural snapshot of a hash map, independent of pointer values
// ---------------------------------------------------------------------------

/// How element keys should be interpreted when snapshotting.
///
/// `cmp_end` bounds the element bytes that are *deterministic*: bytes beyond
/// it are struct padding that neither library ever writes, so they hold
/// whatever `realloc` returned and must not be compared.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum KeyKind {
    /// Key is `ks` raw bytes at offset 0; compare `[0, cmp_end)`.
    Bytes { ks: usize, cmp_end: usize },
    /// Key is a `char *` at `off`; compare the pointed-to string plus the
    /// element bytes `[0, off)` and `[off + 8, cmp_end)`.
    StrPtr { off: usize, cmp_end: usize },
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct TableSnap {
    pub slot_count: usize,
    pub used_count: usize,
    pub used_count_threshold: usize,
    pub used_count_shrink_threshold: usize,
    pub tombstone_count: usize,
    pub tombstone_count_threshold: usize,
    pub seed: usize,
    pub slot_count_log2: usize,
    pub arena_remaining: usize,
    pub arena_block: u8,
    pub arena_mode: u8,
    pub arena_has_storage: bool,
    pub buckets: Vec<(usize, isize)>,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct MapSnap {
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub table: Option<TableSnap>,
    /// One entry per array slot `0..length` (index 0 is the default element):
    /// (key rendering, remaining element bytes after the key field).
    pub elems: Vec<(Option<Vec<u8>>, Vec<u8>)>,
}

unsafe fn cstr_bytes(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        None
    } else {
        Some(CStr::from_ptr(p).to_bytes().to_vec())
    }
}

/// Snapshot the map reachable from the user-visible pointer `t`
/// (`t == raw_array + elemsize`).
pub unsafe fn snap(t: *mut c_void, elemsize: usize, kind: KeyKind) -> MapSnap {
    assert!(!t.is_null(), "snap() on a NULL map");
    let raw = (t as *mut u8).sub(elemsize);
    let h = raw.sub(HEADER_SIZE) as *mut ArrayHeader;
    let length = (*h).length;
    let capacity = (*h).capacity;
    let temp = (*h).temp;

    let table = if (*h).hash_table.is_null() {
        None
    } else {
        let ti = (*h).hash_table as *mut HashIndex;
        let str_mode = (*ti).string.mode;
        let nb = (*ti).slot_count >> BUCKET_SHIFT;
        let mut buckets = Vec::with_capacity(nb * BUCKET_LENGTH);
        for b in 0..nb {
            let bp = (*ti).storage.add(b);
            for j in 0..BUCKET_LENGTH {
                buckets.push(((*bp).hash[j], (*bp).index[j]));
            }
        }
        Some(TableSnap {
            slot_count: (*ti).slot_count,
            used_count: (*ti).used_count,
            used_count_threshold: (*ti).used_count_threshold,
            used_count_shrink_threshold: (*ti).used_count_shrink_threshold,
            tombstone_count: (*ti).tombstone_count,
            tombstone_count_threshold: (*ti).tombstone_count_threshold,
            seed: (*ti).seed,
            slot_count_log2: (*ti).slot_count_log2,
            arena_remaining: (*ti).string.remaining,
            arena_block: (*ti).string.block,
            arena_mode: str_mode,
            arena_has_storage: !(*ti).string.storage.is_null(),
            buckets,
        })
    };

    // Element rendering.
    let str_mode = table.as_ref().map(|t| t.arena_mode).unwrap_or(0);
    let deref_keys = matches!(kind, KeyKind::StrPtr { .. })
        && (str_mode == 1 || str_mode == 2 || str_mode == 3);
    let mut elems = Vec::with_capacity(length);
    for i in 0..length {
        let e = raw.add(elemsize * i);
        match kind {
            KeyKind::Bytes { ks, cmp_end } => {
                let key = std::slice::from_raw_parts(e, ks).to_vec();
                let tail = std::slice::from_raw_parts(e.add(ks), cmp_end - ks).to_vec();
                elems.push((Some(key), tail));
            }
            KeyKind::StrPtr { off, cmp_end } => {
                let kp = *(e.add(off) as *const *const c_char);
                // The default element (index 0) is memset to zero, so its key
                // pointer is NULL.
                let key = if deref_keys && i != 0 { cstr_bytes(kp) } else { None };
                let mut tail = Vec::new();
                tail.extend_from_slice(std::slice::from_raw_parts(e, off));
                tail.extend_from_slice(std::slice::from_raw_parts(
                    e.add(off + 8),
                    cmp_end - off - 8,
                ));
                elems.push((key, tail));
            }
        }
    }

    MapSnap { length, capacity, temp, table, elems }
}

/// Snapshot of a bare dynamic array (no hash table involved).
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct ArrSnap {
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub has_table: bool,
    pub payload: Vec<u8>,
}

pub unsafe fn arr_snap(a: *mut c_void, payload_len: usize) -> ArrSnap {
    let h = (a as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader;
    ArrSnap {
        length: (*h).length,
        capacity: (*h).capacity,
        temp: (*h).temp,
        has_table: !(*h).hash_table.is_null(),
        payload: std::slice::from_raw_parts(a as *const u8, payload_len).to_vec(),
    }
}

/// `stbds_temp_key(raw)`: `*(char **) stbds_header(raw)->hash_table`, i.e. the
/// `temp_key` field of the hash index.  Only meaningful right after a
/// string-mode `stbds_hmput_key`; it is *uninitialised memory* otherwise, so
/// it is deliberately NOT part of `TableSnap`.
pub unsafe fn map_temp_key(t: *mut c_void, elemsize: usize) -> Option<Vec<u8>> {
    let raw = (t as *mut u8).sub(elemsize);
    let h = raw.sub(HEADER_SIZE) as *mut ArrayHeader;
    if (*h).hash_table.is_null() {
        return None;
    }
    cstr_bytes(*((*h).hash_table as *const *const c_char))
}

/// `stbds_temp(raw)` where `t` is the user pointer.
pub unsafe fn map_temp(t: *mut c_void, elemsize: usize) -> isize {
    let raw = (t as *mut u8).sub(elemsize);
    (*(raw.sub(HEADER_SIZE) as *mut ArrayHeader)).temp
}

pub unsafe fn map_len(t: *mut c_void, elemsize: usize) -> usize {
    let raw = (t as *mut u8).sub(elemsize);
    (*(raw.sub(HEADER_SIZE) as *mut ArrayHeader)).length
}

/// Read the `int` value field at `off` in element `i` (user-visible indexing:
/// `t[i]`, so array slot `i + 1`).
pub unsafe fn elem_i32(t: *mut c_void, elemsize: usize, i: isize, off: usize) -> i32 {
    let p = (t as *mut u8).offset(i * elemsize as isize).add(off);
    (p as *const i32).read_unaligned()
}

pub unsafe fn set_elem_i32(t: *mut c_void, elemsize: usize, i: isize, off: usize, v: i32) {
    let p = (t as *mut u8).offset(i * elemsize as isize).add(off);
    (p as *mut i32).write_unaligned(v);
}

// ---------------------------------------------------------------------------
// Arena snapshot
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct ArenaSnap {
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
    pub has_storage: bool,
    pub block_chain_len: usize,
}

pub unsafe fn arena_snap(a: *const StringArena) -> ArenaSnap {
    let mut n = 0usize;
    let mut p = (*a).storage as *const *const c_void;
    while !p.is_null() {
        n += 1;
        if n > 100_000 {
            break;
        }
        p = *p as *const *const c_void;
    }
    ArenaSnap {
        remaining: (*a).remaining,
        block: (*a).block,
        mode: (*a).mode,
        has_storage: !(*a).storage.is_null(),
        block_chain_len: n,
    }
}

// ---------------------------------------------------------------------------
// stdout capture (for the printf inside sh_geti)
// ---------------------------------------------------------------------------

const O_RDWR: c_int = 2;
const O_CREAT: c_int = 64;
const O_TRUNC: c_int = 512;

/// Run `f` with fd 1 redirected to a temporary file and return everything it
/// wrote.
pub unsafe fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    let path = format!("{dir}/cap_{}_{}.txt", tag, std::process::id());
    let cpath = CString::new(path.clone()).unwrap();

    fflush(std::ptr::null_mut());
    let saved = dup(1);
    assert!(saved >= 0, "dup(1) failed");
    let fd = open(cpath.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int);
    assert!(fd >= 0, "open({path}) failed");
    dup2(fd, 1);

    f();

    fflush(std::ptr::null_mut());
    dup2(saved, 1);
    close(saved);

    lseek(fd, 0, 0);
    let mut out = Vec::new();
    let mut buf = vec![0u8; 65536];
    loop {
        let n = read(fd, buf.as_mut_ptr() as *mut c_void, buf.len());
        if n <= 0 {
            break;
        }
        out.extend_from_slice(&buf[..n as usize]);
    }
    close(fd);
    let _ = std::fs::remove_file(&path);
    out
}

// ---------------------------------------------------------------------------
// Map driver: reimplements the stb_ds macros on top of the exported functions
// ---------------------------------------------------------------------------

/// Mirrors the `stbds_hm*` / `stbds_sh*` macros for one loaded library.
pub struct Map<'a> {
    pub api: &'a Api,
    /// user-visible pointer, `raw_array + elemsize`; NULL until first use
    pub t: *mut c_void,
    pub elemsize: usize,
    pub keysize: usize,
    pub keyoffset: usize,
    /// `mode` argument passed to the `stbds_hm*_key` functions
    pub mode: c_int,
}

impl<'a> Map<'a> {
    pub fn new(api: &'a Api, elemsize: usize, keysize: usize, mode: c_int) -> Map<'a> {
        Map { api, t: std::ptr::null_mut(), elemsize, keysize, keyoffset: 0, mode }
    }

    /// `sh_new_strdup` / `sh_new_arena` / any `stbds_shmode_func` mode.
    pub unsafe fn shmode(api: &'a Api, elemsize: usize, keysize: usize, mode: c_int, sh: c_int) -> Map<'a> {
        let t = (api.shmode_func)(elemsize, sh);
        Map { api, t, elemsize, keysize, keyoffset: 0, mode }
    }

    /// `stbds_hmgeti` / `stbds_shgeti`
    pub unsafe fn geti(&mut self, key: *mut c_void) -> isize {
        self.t = (self.api.hmget_key)(self.t, self.elemsize, key, self.keysize, self.mode);
        map_temp(self.t, self.elemsize)
    }

    /// `stbds_hmgeti_ts` / low-level `stbds_hmget_key_ts`
    pub unsafe fn geti_ts(&mut self, key: *mut c_void) -> isize {
        let mut temp: isize = i64::MIN as isize;
        self.t =
            (self.api.hmget_key_ts)(self.t, self.elemsize, key, self.keysize, &mut temp, self.mode);
        temp
    }

    /// `stbds_hmput_key` half of `stbds_hmput`/`stbds_shput`; returns `temp`.
    pub unsafe fn put_key(&mut self, key: *mut c_void) -> isize {
        self.t = (self.api.hmput_key)(self.t, self.elemsize, key, self.keysize, self.mode);
        map_temp(self.t, self.elemsize)
    }

    /// `stbds_hmdel` / `stbds_shdel`
    pub unsafe fn del(&mut self, key: *mut c_void) -> isize {
        self.t = (self.api.hmdel_key)(
            self.t,
            self.elemsize,
            key,
            self.keysize,
            self.keyoffset,
            self.mode,
        );
        if self.t.is_null() {
            0
        } else {
            map_temp(self.t, self.elemsize)
        }
    }

    /// `stbds_hmdefault` value part is left to the caller; this is the
    /// `stbds_hmput_default` call itself.
    pub unsafe fn put_default(&mut self) {
        self.t = (self.api.hmput_default)(self.t, self.elemsize);
    }

    /// `stbds_hmlen`
    pub unsafe fn len(&self) -> isize {
        if self.t.is_null() {
            0
        } else {
            map_len(self.t, self.elemsize) as isize - 1
        }
    }

    pub unsafe fn snap(&self, kind: KeyKind) -> MapSnap {
        snap(self.t, self.elemsize, kind)
    }

    /// `stbds_hmfree` / `stbds_shfree`
    pub unsafe fn free(&mut self) {
        if !self.t.is_null() {
            (self.api.hmfree_func)((self.t as *mut u8).sub(self.elemsize) as *mut c_void, self.elemsize);
            self.t = std::ptr::null_mut();
        }
    }

    /// write an `i32` at `off` inside `t[i]`
    pub unsafe fn set_i32(&self, i: isize, off: usize, v: i32) {
        set_elem_i32(self.t, self.elemsize, i, off, v);
    }
    pub unsafe fn get_i32(&self, i: isize, off: usize) -> i32 {
        elem_i32(self.t, self.elemsize, i, off)
    }
}

// ---------------------------------------------------------------------------
// Abort detection: run a closure in a forked child and report how it died
// ---------------------------------------------------------------------------

extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Outcome {
    /// closure returned normally
    Ok,
    /// killed by signal `n` (6 == SIGABRT from a failed `assert`)
    Signal(c_int),
    /// exited with a non-zero status
    Exit(c_int),
}

/// Run `f` in a forked child so that a failing `assert()` (which calls
/// `abort()`) can be observed instead of taking the test process down.
pub unsafe fn outcome_of<F: FnOnce()>(f: F) -> Outcome {
    fflush(std::ptr::null_mut());
    let pid = fork();
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        f();
        fflush(std::ptr::null_mut());
        _exit(0);
    }
    let mut status: c_int = 0;
    let r = waitpid(pid, &mut status, 0);
    assert_eq!(r, pid, "waitpid failed");
    let sig = status & 0x7f;
    if sig != 0 {
        Outcome::Signal(sig)
    } else {
        let code = (status >> 8) & 0xff;
        if code == 0 {
            Outcome::Ok
        } else {
            Outcome::Exit(code)
        }
    }
}

pub const SIGABRT: c_int = 6;

//! Differential-test harness.
//!
//! Loads BOTH shared libraries (the C one produced by `c_src/CMakeLists.txt`
//! and the Rust `cdylib`) with `libloading` and calls every entry point through
//! its exported C symbol, so the `#[no_mangle]`/`extern "C"` wrappers are part
//! of what is being tested.  No Rust function is ever called directly.

#![allow(dead_code)]

use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// libc bits the harness itself needs
// ---------------------------------------------------------------------------
extern "C" {
    pub fn malloc(n: usize) -> *mut c_void;
    pub fn free(p: *mut c_void);
    pub fn fflush(f: *mut c_void) -> c_int;
    pub fn fork() -> c_int;
    pub fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    pub fn _exit(code: c_int) -> !;
    pub fn dup(fd: c_int) -> c_int;
    pub fn dup2(old: c_int, new: c_int) -> c_int;
    pub fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    pub fn close(fd: c_int) -> c_int;
}

// ---------------------------------------------------------------------------
// Layout-identical mirrors of the C structs (only used to *read* library state)
// ---------------------------------------------------------------------------

pub const HEADER_SIZE: usize = 32;
pub const BUCKET_LENGTH: usize = 8;
pub const BUCKET_SHIFT: usize = 3;
pub const BUCKET_MASK: usize = 7;

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;

pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

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
    pub fn new() -> Self {
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

const _: () = assert!(std::mem::size_of::<ArrayHeader>() == 32);
const _: () = assert!(std::mem::size_of::<HashBucket>() == 128);
const _: () = assert!(std::mem::size_of::<HashIndex>() == 104);
const _: () = assert!(std::mem::size_of::<StringArena>() == 24);

// ---------------------------------------------------------------------------
// The exported ABI
// ---------------------------------------------------------------------------

pub type FnArrGrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrFreef = unsafe extern "C" fn(*mut c_void);
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnStrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
pub type FnStrReset = unsafe extern "C" fn(*mut StringArena);
pub type FnHmFreeFunc = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmGetKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmGetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type FnHmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmPutKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnShModeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnStrkey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnShPuts = unsafe extern "C" fn(c_int);

pub struct Api {
    pub name: &'static str,
    pub arrgrowf: FnArrGrowf,
    pub arrfreef: FnArrFreef,
    pub rand_seed: FnRandSeed,
    pub hash_bytes: FnHashBytes,
    pub hash_string: FnHashString,
    pub stralloc: FnStrAlloc,
    pub strreset: FnStrReset,
    pub hmfree_func: FnHmFreeFunc,
    pub hmget_key: FnHmGetKey,
    pub hmget_key_ts: FnHmGetKeyTs,
    pub hmput_default: FnHmPutDefault,
    pub hmput_key: FnHmPutKey,
    pub hmdel_key: FnHmDelKey,
    pub shmode_func: FnShModeFunc,
    pub strkey: FnStrkey,
    pub sh_puts: FnShPuts,
}

unsafe fn sym<T: Copy>(lib: &Library, name: &str) -> T {
    let s: libloading::Symbol<T> = lib
        .get(format!("{name}\0").as_bytes())
        .unwrap_or_else(|e| panic!("missing symbol `{name}`: {e}"));
    *s
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("TR_C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("TR_RUST_SO") {
        return PathBuf::from(p);
    }
    let rel = manifest_dir().join("target/release/libsh_puts_lib.so");
    if rel.exists() {
        return rel;
    }
    manifest_dir().join("target/debug/libsh_puts_lib.so")
}

unsafe fn load(name: &'static str, path: &PathBuf) -> Api {
    let lib = Library::new(path)
        .unwrap_or_else(|e| panic!("cannot dlopen {} ({}): {e}", path.display(), name));
    let api = Api {
        name,
        arrgrowf: sym(&lib, "stbds_arrgrowf"),
        arrfreef: sym(&lib, "stbds_arrfreef"),
        rand_seed: sym(&lib, "stbds_rand_seed"),
        hash_bytes: sym(&lib, "stbds_hash_bytes"),
        hash_string: sym(&lib, "stbds_hash_string"),
        stralloc: sym(&lib, "stbds_stralloc"),
        strreset: sym(&lib, "stbds_strreset"),
        hmfree_func: sym(&lib, "stbds_hmfree_func"),
        hmget_key: sym(&lib, "stbds_hmget_key"),
        hmget_key_ts: sym(&lib, "stbds_hmget_key_ts"),
        hmput_default: sym(&lib, "stbds_hmput_default"),
        hmput_key: sym(&lib, "stbds_hmput_key"),
        hmdel_key: sym(&lib, "stbds_hmdel_key"),
        shmode_func: sym(&lib, "stbds_shmode_func"),
        strkey: sym(&lib, "strkey"),
        sh_puts: sym(&lib, "sh_puts"),
    };
    // The library must stay mapped for the whole test-binary lifetime.
    std::mem::forget(lib);
    api
}

pub fn apis() -> &'static (Api, Api) {
    static A: OnceLock<(Api, Api)> = OnceLock::new();
    A.get_or_init(|| unsafe { (load("C", &c_so_path()), load("RUST", &rust_so_path())) })
}

/// `(c_api, rust_api)`
pub fn pair() -> (&'static Api, &'static Api) {
    let p = apis();
    (&p.0, &p.1)
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) — fixed seed per test for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9e37_79b9_7f4a_7c15)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// uniform-ish in `0..n`
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
    /// NUL-free bytes in `lo..=hi`
    pub fn cbytes(&mut self, n: usize, lo: u8, hi: u8) -> Vec<u8> {
        let span = (hi - lo) as u64 + 1;
        (0..n)
            .map(|_| lo + ((self.next_u64() >> 24) % span) as u8)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Map accessors (the stb_ds macros, spelled out)
// ---------------------------------------------------------------------------

/// `STBDS_HASH_TO_ARR(t, elemsize)` == `t - 1` element
pub unsafe fn raw_of(t: *mut c_void, elemsize: usize) -> *mut c_void {
    (t as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

/// `stbds_header(raw)`
pub unsafe fn header_of_raw(raw: *mut c_void) -> *mut ArrayHeader {
    (raw as *mut u8).wrapping_sub(HEADER_SIZE) as *mut ArrayHeader
}

/// `stbds_header(t - 1)` for a map pointer `t`
pub unsafe fn map_header(t: *mut c_void, elemsize: usize) -> *mut ArrayHeader {
    header_of_raw(raw_of(t, elemsize))
}

/// `stbds_temp(t-1)`
pub unsafe fn map_temp(t: *mut c_void, elemsize: usize) -> isize {
    (*map_header(t, elemsize)).temp
}

/// `stbds_hmlen(t)`
pub unsafe fn map_len(t: *mut c_void, elemsize: usize) -> isize {
    if t.is_null() {
        0
    } else {
        (*map_header(t, elemsize)).length as isize - 1
    }
}

pub unsafe fn map_table(t: *mut c_void, elemsize: usize) -> *mut HashIndex {
    (*map_header(t, elemsize)).hash_table as *mut HashIndex
}

/// address of user element `idx` (`idx == -1` is the default/sentinel element)
pub unsafe fn map_elem(t: *mut c_void, elemsize: usize, idx: isize) -> *mut u8 {
    (t as *mut u8).wrapping_offset(idx * elemsize as isize)
}

pub unsafe fn cstr(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        return None;
    }
    let mut v = Vec::new();
    let mut q = p as *const u8;
    loop {
        let b = *q;
        if b == 0 {
            break;
        }
        v.push(b);
        q = q.add(1);
    }
    Some(v)
}

// ---------------------------------------------------------------------------
// Deep snapshot of a map, comparable across libraries
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TableSnap {
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
    pub string_has_storage: bool,
    /// bucket-array contents: (hash[8], index[8]) per bucket
    pub buckets: Vec<([usize; 8], [isize; 8])>,
    /// `storage` must be 64-byte aligned (address itself is not comparable)
    pub storage_aligned: bool,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MapSnap {
    pub is_null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub table: Option<TableSnap>,
    /// raw bytes of raw elements `0..length` (raw 0 == the default element)
    pub raw_elems: Vec<Vec<u8>>,
    /// when the element's first 8 bytes are a `char *` key: its NUL-terminated
    /// content (pointers themselves are not comparable across libraries)
    pub key_strings: Option<Vec<Option<Vec<u8>>>>,
}

/// `keys_are_pointers`: interpret bytes `[0..8)` of each raw element as a
/// `char *` and compare the pointed-to string instead of the pointer value.
pub unsafe fn snap(t: *mut c_void, elemsize: usize, keys_are_pointers: bool) -> MapSnap {
    if t.is_null() {
        return MapSnap {
            is_null: true,
            length: 0,
            capacity: 0,
            temp: 0,
            table: None,
            raw_elems: vec![],
            key_strings: None,
        };
    }
    let raw = raw_of(t, elemsize);
    let h = header_of_raw(raw);
    let length = (*h).length;
    let table = map_table(t, elemsize);

    let table_snap = if table.is_null() {
        None
    } else {
        let tt = &*table;
        let nbuckets = tt.slot_count >> BUCKET_SHIFT;
        let mut buckets = Vec::with_capacity(nbuckets);
        for i in 0..nbuckets {
            let b = &*tt.storage.add(i);
            buckets.push((b.hash, b.index));
        }
        Some(TableSnap {
            slot_count: tt.slot_count,
            used_count: tt.used_count,
            used_count_threshold: tt.used_count_threshold,
            used_count_shrink_threshold: tt.used_count_shrink_threshold,
            tombstone_count: tt.tombstone_count,
            tombstone_count_threshold: tt.tombstone_count_threshold,
            seed: tt.seed,
            slot_count_log2: tt.slot_count_log2,
            string_remaining: tt.string.remaining,
            string_block: tt.string.block,
            string_mode: tt.string.mode,
            string_has_storage: !tt.string.storage.is_null(),
            buckets,
            storage_aligned: (tt.storage as usize) % 64 == 0,
        })
    };

    let mut raw_elems = Vec::with_capacity(length);
    let mut key_strings = if keys_are_pointers { Some(Vec::new()) } else { None };
    for i in 0..length {
        let p = (raw as *mut u8).add(i * elemsize);
        if keys_are_pointers {
            // skip the (incomparable) pointer bytes, keep the payload
            let mut v = vec![0u8; elemsize.saturating_sub(8)];
            if elemsize > 8 {
                std::ptr::copy_nonoverlapping(p.add(8), v.as_mut_ptr(), elemsize - 8);
            }
            raw_elems.push(v);
            let kp = *(p as *const *const c_char);
            key_strings.as_mut().unwrap().push(if i == 0 { None } else { cstr(kp) });
        } else {
            let mut v = vec![0u8; elemsize];
            std::ptr::copy_nonoverlapping(p, v.as_mut_ptr(), elemsize);
            raw_elems.push(v);
        }
    }

    MapSnap {
        is_null: false,
        length,
        capacity: (*h).capacity,
        temp: (*h).temp,
        table: table_snap,
        raw_elems,
        key_strings,
    }
}

// ---------------------------------------------------------------------------
// Two-sided map driver: keeps the C map and the Rust map in lock-step
// ---------------------------------------------------------------------------

pub struct DualMap {
    pub elemsize: usize,
    pub c: *mut c_void,
    pub r: *mut c_void,
    /// keys stored as `char *` (string modes)
    pub ptr_keys: bool,
}

impl DualMap {
    pub fn null(elemsize: usize, ptr_keys: bool) -> Self {
        DualMap { elemsize, c: std::ptr::null_mut(), r: std::ptr::null_mut(), ptr_keys }
    }

    pub fn check(&self, ctx: &str) {
        unsafe {
            let a = snap(self.c, self.elemsize, self.ptr_keys);
            let b = snap(self.r, self.elemsize, self.ptr_keys);
            if a != b {
                panic!("map state diverged at {ctx}\n C = {a:#?}\n R = {b:#?}");
            }
            if let Some(ts) = a.table.as_ref() {
                assert!(ts.storage_aligned, "C bucket storage not 64B aligned at {ctx}");
            }
            if let Some(ts) = b.table.as_ref() {
                assert!(ts.storage_aligned, "RUST bucket storage not 64B aligned at {ctx}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// forked-child helper: compare crash / abort behaviour
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Outcome {
    Exited(c_int),
    Signalled(c_int),
}

/// Runs `f` in a forked child; returns how the child terminated plus whatever
/// it wrote to stderr.
pub fn in_child<F: FnOnce()>(f: F) -> (Outcome, Vec<u8>) {
    let path = std::env::temp_dir().join(format!(
        "tr_child_{}_{}.err",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    let cpath = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
    unsafe {
        // O_WRONLY|O_CREAT|O_TRUNC, 0644
        let fd = open(cpath.as_ptr(), 0o1 | 0o100 | 0o1000, 0o644 as c_int);
        assert!(fd >= 0, "cannot create {}", path.display());
        let pid = fork();
        if pid == 0 {
            dup2(fd, 2);
            close(fd);
            f();
            fflush(std::ptr::null_mut());
            _exit(0);
        }
        close(fd);
        let mut status: c_int = 0;
        let w = waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed");
        let err = std::fs::read(&path).unwrap_or_default();
        let _ = std::fs::remove_file(&path);
        let outcome = if status & 0x7f == 0x7f {
            Outcome::Signalled(-1) // stopped; should not happen
        } else if status & 0x7f != 0 {
            Outcome::Signalled(status & 0x7f)
        } else {
            Outcome::Exited((status >> 8) & 0xff)
        };
        (outcome, err)
    }
}

static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

// ---------------------------------------------------------------------------
// stdout capture (for `sh_puts`)
// ---------------------------------------------------------------------------

pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let path = std::env::temp_dir().join(format!(
        "tr_out_{}_{}.txt",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    let cpath = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0);
        let fd = open(cpath.as_ptr(), 0o1 | 0o100 | 0o1000, 0o644 as c_int);
        assert!(fd >= 0, "cannot create {}", path.display());
        assert!(dup2(fd, 1) >= 0);
        close(fd);
        f();
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0);
        close(saved);
    }
    let out = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    out
}

/// heap buffer holding a NUL-terminated key, so both libraries see the exact
/// same bytes at the exact same address
pub struct CBuf {
    pub p: *mut u8,
    pub len: usize,
}

impl CBuf {
    pub fn new(bytes: &[u8]) -> Self {
        unsafe {
            let p = malloc(bytes.len().max(1)) as *mut u8;
            assert!(!p.is_null());
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len());
            CBuf { p, len: bytes.len() }
        }
    }
    /// NUL-terminated copy of `s`
    pub fn cstr(s: &[u8]) -> Self {
        let mut v = s.to_vec();
        v.push(0);
        Self::new(&v)
    }
    pub fn as_void(&self) -> *mut c_void {
        self.p as *mut c_void
    }
    pub fn as_char(&self) -> *mut c_char {
        self.p as *mut c_char
    }
}

impl Drop for CBuf {
    fn drop(&mut self) {
        unsafe { free(self.p as *mut c_void) }
    }
}

// ---------------------------------------------------------------------------
// `Dual` — drives the C map and the Rust map through the exported ABI in
// lock-step and deep-compares them after every operation.
// ---------------------------------------------------------------------------

pub struct Dual {
    pub elemsize: usize,
    pub ptr_keys: bool,
    pub c: *mut c_void,
    pub r: *mut c_void,
    /// keeps caller-owned key buffers alive (SH_DEFAULT borrows them)
    pub keep: Vec<CBuf>,
    pub checks: usize,
}

impl Dual {
    pub fn new(elemsize: usize, ptr_keys: bool) -> Self {
        Dual {
            elemsize,
            ptr_keys,
            c: std::ptr::null_mut(),
            r: std::ptr::null_mut(),
            keep: Vec::new(),
            checks: 0,
        }
    }

    /// `sh_new_arena` / `sh_new_strdup` / `stbds_shmode_func`
    pub fn shmode(&mut self, mode: c_int) {
        let (c, r) = pair();
        unsafe {
            self.c = (c.shmode_func)(self.elemsize, mode);
            self.r = (r.shmode_func)(self.elemsize, mode);
        }
        self.check(&format!("shmode({mode})"));
    }

    pub fn check(&mut self, ctx: &str) {
        self.checks += 1;
        unsafe {
            let a = snap(self.c, self.elemsize, self.ptr_keys);
            let b = snap(self.r, self.elemsize, self.ptr_keys);
            if a != b {
                panic!(
                    "map diverged after {ctx}\n--- C ---\n{a:#?}\n--- RUST ---\n{b:#?}"
                );
            }
            if let Some(ts) = a.table.as_ref() {
                assert!(ts.storage_aligned, "C bucket storage misaligned after {ctx}");
            }
            if let Some(ts) = b.table.as_ref() {
                assert!(ts.storage_aligned, "RUST bucket storage misaligned after {ctx}");
            }
        }
    }

    /// `stbds_hmput_key` + the macro's element store (writes the WHOLE element
    /// so that no padding byte is left uninitialised).
    /// Returns the index reported through `header->temp`.
    pub fn put_bin(&mut self, key: &[u8], keysize: usize, payload: &[u8], mode: c_int) -> (isize, isize) {
        let (c, r) = pair();
        let kb = CBuf::new(key);
        let es = self.elemsize;
        assert_eq!(key.len() + payload.len(), es, "element must be fully written");
        unsafe {
            self.c = (c.hmput_key)(self.c, es, kb.as_void(), keysize, mode);
            self.r = (r.hmput_key)(self.r, es, kb.as_void(), keysize, mode);
            let tc = map_temp(self.c, es);
            let tr = map_temp(self.r, es);
            for (t, i) in [(self.c, tc), (self.r, tr)] {
                let p = map_elem(t, es, i);
                std::ptr::copy_nonoverlapping(key.as_ptr(), p, key.len());
                if !payload.is_empty() {
                    std::ptr::copy_nonoverlapping(payload.as_ptr(), p.add(key.len()), payload.len());
                }
            }
            (tc, tr)
        }
    }

    /// `stbds_shput`: string key, value written after the 8-byte key pointer.
    pub fn put_str(&mut self, key: &[u8], payload: &[u8], mode: c_int) -> (isize, isize) {
        let (c, r) = pair();
        let kb = CBuf::cstr(key);
        let es = self.elemsize;
        assert_eq!(payload.len() + 8, es, "element must be fully written");
        unsafe {
            self.c = (c.hmput_key)(self.c, es, kb.as_void(), 8, mode);
            self.r = (r.hmput_key)(self.r, es, kb.as_void(), 8, mode);
            let tc = map_temp(self.c, es);
            let tr = map_temp(self.r, es);
            if !payload.is_empty() {
                for (t, i) in [(self.c, tc), (self.r, tr)] {
                    let p = map_elem(t, es, i);
                    std::ptr::copy_nonoverlapping(payload.as_ptr(), p.add(8), payload.len());
                }
            }
            self.keep.push(kb);
            (tc, tr)
        }
    }

    /// `stbds_shputs`: whole-struct store followed by `key = stbds_temp_key`.
    pub fn puts_str(&mut self, key: &[u8], payload: &[u8], mode: c_int) -> (isize, isize) {
        let (c, r) = pair();
        let kb = CBuf::cstr(key);
        let es = self.elemsize;
        assert_eq!(payload.len() + 8, es, "element must be fully written");
        unsafe {
            self.c = (c.hmput_key)(self.c, es, kb.as_void(), 8, mode);
            self.r = (r.hmput_key)(self.r, es, kb.as_void(), 8, mode);
            let tc = map_temp(self.c, es);
            let tr = map_temp(self.r, es);
            for (t, i) in [(self.c, tc), (self.r, tr)] {
                let p = map_elem(t, es, i);
                // t[temp] = s
                std::ptr::copy_nonoverlapping(kb.p, p, 8);
                if !payload.is_empty() {
                    std::ptr::copy_nonoverlapping(payload.as_ptr(), p.add(8), payload.len());
                }
                // t[temp].key = stbds_temp_key(t-1)
                let tk = *((*map_header(t, es)).hash_table as *mut *mut c_char);
                *(p as *mut *mut c_char) = tk;
            }
            self.keep.push(kb);
            (tc, tr)
        }
    }

    /// `stbds_hmgeti` / `stbds_shgeti`
    pub fn get(&mut self, key: &[u8], keysize: usize, mode: c_int, nul_terminate: bool) -> (isize, isize) {
        let (c, r) = pair();
        let kb = if nul_terminate { CBuf::cstr(key) } else { CBuf::new(key) };
        let es = self.elemsize;
        unsafe {
            self.c = (c.hmget_key)(self.c, es, kb.as_void(), keysize, mode);
            self.r = (r.hmget_key)(self.r, es, kb.as_void(), keysize, mode);
            (map_temp(self.c, es), map_temp(self.r, es))
        }
    }

    /// `stbds_hmgeti_ts`
    pub fn get_ts(&mut self, key: &[u8], keysize: usize, mode: c_int, nul_terminate: bool) -> (isize, isize) {
        let (c, r) = pair();
        let kb = if nul_terminate { CBuf::cstr(key) } else { CBuf::new(key) };
        let es = self.elemsize;
        let mut a: isize = 0x5a5a;
        let mut b: isize = 0x5a5a;
        unsafe {
            self.c = (c.hmget_key_ts)(self.c, es, kb.as_void(), keysize, &mut a, mode);
            self.r = (r.hmget_key_ts)(self.r, es, kb.as_void(), keysize, &mut b, mode);
        }
        (a, b)
    }

    /// `stbds_hmdel` / `stbds_shdel`
    pub fn del(
        &mut self,
        key: &[u8],
        keysize: usize,
        keyoffset: usize,
        mode: c_int,
        nul_terminate: bool,
    ) -> (isize, isize) {
        let (c, r) = pair();
        let kb = if nul_terminate { CBuf::cstr(key) } else { CBuf::new(key) };
        let es = self.elemsize;
        unsafe {
            self.c = (c.hmdel_key)(self.c, es, kb.as_void(), keysize, keyoffset, mode);
            self.r = (r.hmdel_key)(self.r, es, kb.as_void(), keysize, keyoffset, mode);
            let a = if self.c.is_null() { 0 } else { map_temp(self.c, es) };
            let b = if self.r.is_null() { 0 } else { map_temp(self.r, es) };
            (a, b)
        }
    }

    /// `stbds_hmdefault`: `t = hmput_default(t); t[-1] = value`
    pub fn put_default(&mut self, elem: &[u8]) {
        let (c, r) = pair();
        let es = self.elemsize;
        assert_eq!(elem.len(), es);
        unsafe {
            self.c = (c.hmput_default)(self.c, es);
            self.r = (r.hmput_default)(self.r, es);
            for t in [self.c, self.r] {
                std::ptr::copy_nonoverlapping(elem.as_ptr(), map_elem(t, es, -1), es);
            }
        }
    }

    pub fn len(&self) -> (isize, isize) {
        unsafe { (map_len(self.c, self.elemsize), map_len(self.r, self.elemsize)) }
    }

    /// `stbds_hmfree`
    pub fn free(&mut self) {
        let (c, r) = pair();
        let es = self.elemsize;
        unsafe {
            if !self.c.is_null() {
                (c.hmfree_func)(raw_of(self.c, es), es);
            }
            if !self.r.is_null() {
                (r.hmfree_func)(raw_of(self.r, es), es);
            }
        }
        self.c = std::ptr::null_mut();
        self.r = std::ptr::null_mut();
        self.keep.clear();
    }
}

/// little-endian bytes of an i32 / i64, as C would store them
pub fn le32(v: i32) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}
pub fn le64(v: i64) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}

/// both libraries must start from the same global hash seed
pub fn sync_seed(seed: usize) {
    let (c, r) = pair();
    unsafe {
        (c.rand_seed)(seed);
        (r.rand_seed)(seed);
    }
}

/// Serialises tests that touch the libraries' global mutable state
/// (`stbds_hash_seed`, the `strkey` static buffer, `stdout`).
pub fn lock() -> std::sync::MutexGuard<'static, ()> {
    static M: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
    M.get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

impl Rng {
    /// `cbytes` with a randomised length in `minlen .. minlen+span`
    pub fn cbytes_len(&mut self, minlen: usize, span: usize, lo: u8, hi: u8) -> Vec<u8> {
        let n = minlen + self.below(span);
        self.cbytes(n, lo, hi)
    }
    /// `bytes` with a randomised length in `minlen .. minlen+span`
    pub fn bytes_len(&mut self, minlen: usize, span: usize) -> Vec<u8> {
        let n = minlen + self.below(span);
        self.bytes(n)
    }
}

/// Is the Rust `.so` under test a `debug` (i.e. `debug-assertions = on`) build?
pub fn rust_so_is_debug() -> bool {
    let p = rust_so_path();
    p.to_string_lossy().contains("/debug/")
}

/// Compares how the C and the Rust library die on an input that is *undefined
/// behaviour* in the C (a null-pointer dereference).
///
/// The `release` cdylib — the artefact the library ships as — must reproduce the
/// C's fatal signal exactly.  A `debug` cdylib additionally carries rustc's
/// UB checks (`-C debug-assertions`), which detect the null dereference and
/// `panic!` before the load happens; because the panic escapes an
/// `extern "C"` function it becomes `SIGABRT` instead of `SIGSEGV`.  That is a
/// property of the Rust debug profile, not a behavioural difference of the
/// translation, so it is accepted (but still required to be *fatal on the same
/// input*).
pub fn assert_fatal_equivalent(c: &Outcome, r: &Outcome, what: &str) {
    match (c, r) {
        (Outcome::Signalled(a), Outcome::Signalled(b)) if a == b => {}
        (Outcome::Signalled(11), Outcome::Signalled(6)) if rust_so_is_debug() => {}
        _ => panic!("{what}: C died as {c:?} but RUST died as {r:?}"),
    }
}

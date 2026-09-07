// Shared differential-test harness.
//
// Loads BOTH shared objects through `libloading` and exposes one typed wrapper
// per exported symbol.  Nothing in here calls the Rust crate directly: every
// Rust-side call goes through `libhm_geti_lib.so`'s C ABI exports, exactly like
// an external consumer.

#![allow(dead_code)]
#![allow(non_snake_case)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// C ABI types (mirrors of the layouts in c_src/src/lib.c)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct StringArena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

impl StringArena {
    pub fn new() -> Self {
        StringArena {
            storage: std::ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        }
    }
}

pub const HEADER_SIZE: usize = std::mem::size_of::<ArrayHeader>();
pub const BUCKET_LENGTH: usize = 8;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HashBucket {
    pub hash: [usize; BUCKET_LENGTH],
    pub index: [isize; BUCKET_LENGTH],
}

#[repr(C)]
#[derive(Clone, Copy)]
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

pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

// ---------------------------------------------------------------------------
// Function-pointer types
// ---------------------------------------------------------------------------

type FnArrGrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type FnArrFreef = unsafe extern "C" fn(*mut c_void);
type FnRandSeed = unsafe extern "C" fn(usize);
type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
type FnHmFree = unsafe extern "C" fn(*mut c_void, usize);
type FnHmGetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
type FnHmGetKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type FnHmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FnHmPutKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type FnShmodeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
type FnHmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
type FnStralloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
type FnStrreset = unsafe extern "C" fn(*mut StringArena);
type FnStrkey = unsafe extern "C" fn(c_int) -> *mut c_char;
type FnHmGeti = unsafe extern "C" fn(c_int);

/// A loaded implementation (either the C `.so` or the Rust `.so`).
pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub arrgrowf: FnArrGrowf,
    pub arrfreef: FnArrFreef,
    pub rand_seed: FnRandSeed,
    pub hash_string: FnHashString,
    pub hash_bytes: FnHashBytes,
    pub hmfree_func: FnHmFree,
    pub hmget_key_ts: FnHmGetKeyTs,
    pub hmget_key: FnHmGetKey,
    pub hmput_default: FnHmPutDefault,
    pub hmput_key: FnHmPutKey,
    pub shmode_func: FnShmodeFunc,
    pub hmdel_key: FnHmDelKey,
    pub stralloc: FnStralloc,
    pub strreset: FnStrreset,
    pub strkey: FnStrkey,
    pub hm_geti: FnHmGeti,
}

unsafe fn sym<T: Copy>(lib: &Library, n: &[u8]) -> T {
    unsafe {
        let s: Symbol<T> = lib
            .get(n)
            .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(n)));
        *s
    }
}

impl Impl {
    pub fn load(name: &'static str, path: &PathBuf) -> Impl {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("cannot load {}: {e}", path.display()));
            Impl {
                name,
                arrgrowf: sym(&lib, b"stbds_arrgrowf\0"),
                arrfreef: sym(&lib, b"stbds_arrfreef\0"),
                rand_seed: sym(&lib, b"stbds_rand_seed\0"),
                hash_string: sym(&lib, b"stbds_hash_string\0"),
                hash_bytes: sym(&lib, b"stbds_hash_bytes\0"),
                hmfree_func: sym(&lib, b"stbds_hmfree_func\0"),
                hmget_key_ts: sym(&lib, b"stbds_hmget_key_ts\0"),
                hmget_key: sym(&lib, b"stbds_hmget_key\0"),
                hmput_default: sym(&lib, b"stbds_hmput_default\0"),
                hmput_key: sym(&lib, b"stbds_hmput_key\0"),
                shmode_func: sym(&lib, b"stbds_shmode_func\0"),
                hmdel_key: sym(&lib, b"stbds_hmdel_key\0"),
                stralloc: sym(&lib, b"stbds_stralloc\0"),
                strreset: sym(&lib, b"stbds_strreset\0"),
                strkey: sym(&lib, b"strkey\0"),
                hm_geti: sym(&lib, b"hm_geti\0"),
                _lib: lib,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    let build = crate_root().join("..").join("c_src").join("build");
    let mut found: Option<PathBuf> = None;
    for e in std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("c_src/build missing ({e}); run cmake first"))
    {
        let p = e.unwrap().path();
        let f = p.file_name().unwrap().to_string_lossy().to_string();
        if f.starts_with("lib") && f.ends_with(".so") {
            found = Some(p);
        }
    }
    found.expect("no lib*.so in c_src/build")
}

pub fn rust_so_path() -> PathBuf {
    let t = crate_root().join("target");
    let order: [&str; 2] = if cfg!(debug_assertions) {
        ["debug", "release"]
    } else {
        ["release", "debug"]
    };
    for p in order {
        let c = t.join(p).join("libhm_geti_lib.so");
        if c.exists() {
            return c;
        }
    }
    panic!("libhm_geti_lib.so not found; run `cargo build --release`");
}

/// The pair of implementations under differential test.
pub struct Pair {
    pub c: Impl,
    pub r: Impl,
}

pub fn pair() -> Pair {
    Pair {
        c: Impl::load("C", &c_so_path()),
        r: Impl::load("RUST", &rust_so_path()),
    }
}

// ---------------------------------------------------------------------------
// Raw state inspection (pure memory reads — no calls into either library)
// ---------------------------------------------------------------------------

/// `stbds_header(t)`
pub unsafe fn header(arr: *mut c_void) -> ArrayHeader {
    unsafe { *((arr as *mut u8).wrapping_sub(HEADER_SIZE) as *mut ArrayHeader) }
}

pub unsafe fn header_mut(arr: *mut c_void) -> *mut ArrayHeader {
    (arr as *mut u8).wrapping_sub(HEADER_SIZE) as *mut ArrayHeader
}

/// `STBDS_HASH_TO_ARR`
pub fn to_arr(hash_side: *mut c_void, elemsize: usize) -> *mut c_void {
    (hash_side as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

/// `STBDS_ARR_TO_HASH`
pub fn to_hash(arr: *mut c_void, elemsize: usize) -> *mut c_void {
    (arr as *mut u8).wrapping_add(elemsize) as *mut c_void
}

/// The `temp` field for a hash-side map pointer, i.e. `stbds_temp((t)-1)`.
pub unsafe fn map_temp(hash_side: *mut c_void, elemsize: usize) -> isize {
    unsafe { header(to_arr(hash_side, elemsize)).temp }
}

/// Structural snapshot of a map: everything an external observer can legally
/// see, with all addresses normalised away.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub null: bool,
    pub length: usize,
    pub capacity: usize,
    pub has_table: bool,
    pub temp: isize,
    pub slot_count: usize,
    pub used_count: usize,
    pub used_count_threshold: usize,
    pub used_count_shrink_threshold: usize,
    pub tombstone_count: usize,
    pub tombstone_count_threshold: usize,
    pub seed: usize,
    pub slot_count_log2: usize,
    pub string_mode: u8,
    pub string_block: u8,
    pub string_remaining: usize,
    pub string_has_storage: bool,
    /// bucket hashes and indices (addresses excluded)
    pub buckets: Vec<(usize, isize)>,
}

/// `hash_side` is the pointer the macros hand to user code (`t`).
pub unsafe fn snapshot(hash_side: *mut c_void, elemsize: usize) -> Snapshot {
    unsafe {
        if hash_side.is_null() {
            return Snapshot {
                null: true,
                length: 0,
                capacity: 0,
                has_table: false,
                temp: 0,
                slot_count: 0,
                used_count: 0,
                used_count_threshold: 0,
                used_count_shrink_threshold: 0,
                tombstone_count: 0,
                tombstone_count_threshold: 0,
                seed: 0,
                slot_count_log2: 0,
                string_mode: 0,
                string_block: 0,
                string_remaining: 0,
                string_has_storage: false,
                buckets: Vec::new(),
            };
        }
        let arr = to_arr(hash_side, elemsize);
        let h = header(arr);
        let mut s = Snapshot {
            null: false,
            length: h.length,
            capacity: h.capacity,
            has_table: !h.hash_table.is_null(),
            temp: h.temp,
            slot_count: 0,
            used_count: 0,
            used_count_threshold: 0,
            used_count_shrink_threshold: 0,
            tombstone_count: 0,
            tombstone_count_threshold: 0,
            seed: 0,
            slot_count_log2: 0,
            string_mode: 0,
            string_block: 0,
            string_remaining: 0,
            string_has_storage: false,
            buckets: Vec::new(),
        };
        if !h.hash_table.is_null() {
            let t = &*(h.hash_table as *mut HashIndex);
            s.slot_count = t.slot_count;
            s.used_count = t.used_count;
            s.used_count_threshold = t.used_count_threshold;
            s.used_count_shrink_threshold = t.used_count_shrink_threshold;
            s.tombstone_count = t.tombstone_count;
            s.tombstone_count_threshold = t.tombstone_count_threshold;
            s.seed = t.seed;
            s.slot_count_log2 = t.slot_count_log2;
            s.string_mode = t.string.mode;
            s.string_block = t.string.block;
            s.string_remaining = t.string.remaining;
            s.string_has_storage = !t.string.storage.is_null();
            let nb = t.slot_count >> 3;
            for b in 0..nb {
                let bucket = &*t.storage.wrapping_add(b);
                for j in 0..BUCKET_LENGTH {
                    s.buckets.push((bucket.hash[j], bucket.index[j]));
                }
            }
        }
        s
    }
}

/// Raw element bytes `t[0 .. length-1]` relative to the ARRAY side (index 0 is
/// the default slot `t[-1]`).
pub unsafe fn elems(hash_side: *mut c_void, elemsize: usize) -> Vec<u8> {
    unsafe {
        if hash_side.is_null() {
            return Vec::new();
        }
        let arr = to_arr(hash_side, elemsize);
        let len = header(arr).length;
        let mut v = vec![0u8; len * elemsize];
        std::ptr::copy_nonoverlapping(arr as *const u8, v.as_mut_ptr(), len * elemsize);
        v
    }
}

/// Element bytes with the first `ptr_bytes` bytes of every element blanked out
/// (used for string maps whose key field holds an implementation-chosen
/// pointer, which legitimately differs between the two libraries).
pub unsafe fn elems_masked(hash_side: *mut c_void, elemsize: usize, ptr_bytes: usize) -> Vec<u8> {
    unsafe {
        let mut v = elems(hash_side, elemsize);
        let n = v.len() / elemsize;
        for i in 0..n {
            for b in 0..ptr_bytes {
                v[i * elemsize + b] = 0;
            }
        }
        v
    }
}

/// Reads the `char*` key stored at element `i` of a string map and returns the
/// C string it points at.
pub unsafe fn key_string_at(hash_side: *mut c_void, elemsize: usize, i: usize) -> Option<Vec<u8>> {
    unsafe {
        let arr = to_arr(hash_side, elemsize) as *mut u8;
        let p = *(arr.wrapping_add(i * elemsize) as *mut *mut c_char);
        if p.is_null() {
            return None;
        }
        let mut out = Vec::new();
        let mut q = p as *const u8;
        while *q != 0 {
            out.push(*q);
            q = q.wrapping_add(1);
        }
        Some(out)
    }
}

/// Reads the `temp_key` field of the hash index.
pub unsafe fn temp_key_string(hash_side: *mut c_void, elemsize: usize) -> Option<Vec<u8>> {
    unsafe {
        let h = header(to_arr(hash_side, elemsize));
        if h.hash_table.is_null() {
            return None;
        }
        let p = (*(h.hash_table as *mut HashIndex)).temp_key;
        if p.is_null() {
            return None;
        }
        let mut out = Vec::new();
        let mut q = p as *const u8;
        while *q != 0 {
            out.push(*q);
            q = q.wrapping_add(1);
        }
        Some(out)
    }
}

pub unsafe fn cstr(p: *mut c_char) -> Vec<u8> {
    unsafe {
        let mut out = Vec::new();
        let mut q = p as *const u8;
        while *q != 0 {
            out.push(*q);
            q = q.wrapping_add(1);
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed, reproducible)
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E3779B97F4A7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next_u64() % n as u64) as usize }
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| (self.next_u64() >> 24) as u8).collect()
    }
    pub fn ascii(&mut self, n: usize) -> Vec<u8> {
        (0..n)
            .map(|_| b'a' + (self.next_u64() % 26) as u8)
            .collect()
    }
}

/// A NUL-terminated byte buffer usable as `char*`.
pub fn cstring(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

// ---------------------------------------------------------------------------
// Serialisation: both libraries carry a mutable `stbds_hash_seed` global, so
// tests must not interleave.  Every test takes this lock for its whole body.
// ---------------------------------------------------------------------------

use std::sync::{Mutex, MutexGuard};

pub static LOCK: Mutex<()> = Mutex::new(());

pub fn guard() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Reset both libraries' global hash seed so a row is reproducible.
pub fn setup(p: &Pair, seed: usize) {
    unsafe {
        (p.c.rand_seed)(seed);
        (p.r.rand_seed)(seed);
    }
}

/// Assert equality with a row-tagged message.
#[track_caller]
pub fn diff<T: PartialEq + std::fmt::Debug>(row: &str, what: &str, c: T, r: T) {
    assert!(
        c == r,
        "CONFIG/ERROR row {row}: {what} diverges\n  C    = {c:?}\n  RUST = {r:?}"
    );
}

// ---------------------------------------------------------------------------
// A synthetic consumer of the raw API, mirroring what the stbds_* macros do.
// Identical code drives both libraries.
// ---------------------------------------------------------------------------

pub struct Map<'a> {
    pub im: &'a Impl,
    pub t: *mut c_void, // hash-side pointer (what user code holds)
    pub elemsize: usize,
    pub keysize: usize,
}

impl<'a> Map<'a> {
    pub fn new(im: &'a Impl, elemsize: usize, keysize: usize) -> Map<'a> {
        Map { im, t: std::ptr::null_mut(), elemsize, keysize }
    }

    pub fn temp(&self) -> isize {
        unsafe { map_temp(self.t, self.elemsize) }
    }

    /// `&t[idx]`
    pub fn at(&self, idx: isize) -> *mut u8 {
        (self.t as *mut u8).wrapping_offset(idx * self.elemsize as isize)
    }

    /// `hmput(t, k, v)` — binary mode: the macro stores key then value.
    pub fn put(&mut self, key: &mut [u8], value: &[u8], mode: c_int) -> isize {
        unsafe {
            self.t = (self.im.hmput_key)(
                self.t,
                self.elemsize,
                key.as_mut_ptr() as *mut c_void,
                self.keysize,
                mode,
            );
            let idx = self.temp();
            let e = self.at(idx);
            std::ptr::copy_nonoverlapping(key.as_ptr(), e, self.keysize.min(key.len()));
            if !value.is_empty() {
                let room = self.elemsize - self.keysize;
                std::ptr::copy_nonoverlapping(
                    value.as_ptr(),
                    e.wrapping_add(self.keysize),
                    room.min(value.len()),
                );
            }
            idx
        }
    }

    /// `shput(t, k, v)` — string mode: the macro stores ONLY the value (the key
    /// pointer was already written by `stbds_hmput_key`).
    pub fn sput(&mut self, key: *mut c_char, value: &[u8], mode: c_int) -> isize {
        unsafe {
            self.t = (self.im.hmput_key)(
                self.t,
                self.elemsize,
                key as *mut c_void,
                self.keysize,
                mode,
            );
            let idx = self.temp();
            if !value.is_empty() {
                let e = self.at(idx);
                let room = self.elemsize - self.keysize;
                std::ptr::copy_nonoverlapping(
                    value.as_ptr(),
                    e.wrapping_add(self.keysize),
                    room.min(value.len()),
                );
            }
            idx
        }
    }

    /// `hmgeti(t, k)`
    pub fn geti(&mut self, key: &mut [u8], mode: c_int) -> isize {
        unsafe {
            self.t = (self.im.hmget_key)(
                self.t,
                self.elemsize,
                key.as_mut_ptr() as *mut c_void,
                self.keysize,
                mode,
            );
            self.temp()
        }
    }

    /// `shgeti(t, k)`
    pub fn sgeti(&mut self, key: *mut c_char, mode: c_int) -> isize {
        unsafe {
            self.t =
                (self.im.hmget_key)(self.t, self.elemsize, key as *mut c_void, self.keysize, mode);
            self.temp()
        }
    }

    /// `hmgeti_ts(t, k, temp)`
    pub fn geti_ts(&mut self, key: &mut [u8], mode: c_int) -> isize {
        unsafe {
            let mut temp: isize = 0x5A5A;
            self.t = (self.im.hmget_key_ts)(
                self.t,
                self.elemsize,
                key.as_mut_ptr() as *mut c_void,
                self.keysize,
                &mut temp,
                mode,
            );
            temp
        }
    }

    /// `hmdel(t, k)`
    pub fn del(&mut self, key: &mut [u8], mode: c_int, keyoffset: usize) -> isize {
        unsafe {
            self.t = (self.im.hmdel_key)(
                self.t,
                self.elemsize,
                key.as_mut_ptr() as *mut c_void,
                self.keysize,
                keyoffset,
                mode,
            );
            if self.t.is_null() { 0 } else { self.temp() }
        }
    }

    /// `shdel(t, k)`
    pub fn sdel(&mut self, key: *mut c_char, mode: c_int, keyoffset: usize) -> isize {
        unsafe {
            self.t = (self.im.hmdel_key)(
                self.t,
                self.elemsize,
                key as *mut c_void,
                self.keysize,
                keyoffset,
                mode,
            );
            if self.t.is_null() { 0 } else { self.temp() }
        }
    }

    /// `hmdefault(t, v)`
    pub fn put_default(&mut self, value: &[u8]) {
        unsafe {
            self.t = (self.im.hmput_default)(self.t, self.elemsize);
            let e = self.at(-1);
            let room = self.elemsize - self.keysize;
            std::ptr::copy_nonoverlapping(
                value.as_ptr(),
                e.wrapping_add(self.keysize),
                room.min(value.len()),
            );
        }
    }

    /// `sh_new_arena(t)` / `sh_new_strdup(t)`
    pub fn shmode(&mut self, mode: c_int) {
        unsafe {
            self.t = (self.im.shmode_func)(self.elemsize, mode);
        }
    }

    /// value bytes of element `idx`
    pub fn value_at(&self, idx: isize) -> Vec<u8> {
        unsafe {
            let e = self.at(idx).wrapping_add(self.keysize);
            let n = self.elemsize - self.keysize;
            let mut v = vec![0u8; n];
            std::ptr::copy_nonoverlapping(e as *const u8, v.as_mut_ptr(), n);
            v
        }
    }

    pub fn snap(&self) -> Snapshot {
        unsafe { snapshot(self.t, self.elemsize) }
    }

    pub fn elem_bytes(&self) -> Vec<u8> {
        unsafe { elems(self.t, self.elemsize) }
    }

    pub fn elem_bytes_masked(&self, ptr_bytes: usize) -> Vec<u8> {
        unsafe { elems_masked(self.t, self.elemsize, ptr_bytes) }
    }

    /// `hmfree(t)`
    pub fn free(&mut self) {
        unsafe {
            if !self.t.is_null() {
                (self.im.hmfree_func)(to_arr(self.t, self.elemsize), self.elemsize);
            }
            self.t = std::ptr::null_mut();
        }
    }
}

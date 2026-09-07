//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` through `libloading` and exposes
//! the 16 exported symbols as raw `extern "C"` function pointers. No Rust
//! function is ever called directly — every call crosses the FFI boundary
//! exactly as an external consumer's would, which also exercises the
//! `#[no_mangle]` export wrappers.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};

use libloading::Library;

unsafe extern "C" {
    /// glibc: the usable size of a `malloc`ed block. Both `.so`s use the
    /// process's single glibc allocator, so this exposes whether they asked for
    /// equivalent allocation sizes — an off-by-one in a size formula shows up
    /// here even when the functional behaviour happens to match.
    fn malloc_usable_size(p: *mut c_void) -> usize;
}

/// `malloc_usable_size(p)`, or `None` for NULL.
pub unsafe fn usable_size(p: *mut c_void) -> Option<usize> {
    if p.is_null() {
        None
    } else {
        Some(unsafe { malloc_usable_size(p) })
    }
}

// ---------------------------------------------------------------------------
// Mirrors of the C structs (see c_src/src/lib.c lines 227-344).
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

pub const HDR_SIZE: usize = std::mem::size_of::<ArrayHeader>();

pub const BUCKET_LENGTH: usize = 8;
pub const BUCKET_SHIFT: usize = 3;
pub const BUCKET_MASK: usize = BUCKET_LENGTH - 1;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HashBucket {
    pub hash: [usize; BUCKET_LENGTH],
    pub index: [isize; BUCKET_LENGTH],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StringArena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
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

/// The pointer-free, position-independent part of a `stbds_hash_index`, so that
/// two libraries' tables can be compared even though their heap addresses
/// differ.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableSnapshot {
    pub slot_count: usize,
    pub used_count: usize,
    pub used_count_threshold: usize,
    pub used_count_shrink_threshold: usize,
    pub tombstone_count: usize,
    pub tombstone_count_threshold: usize,
    pub seed: usize,
    pub slot_count_log2: usize,
    pub arena_has_storage: bool,
    pub arena_remaining: usize,
    pub arena_block: u8,
    pub arena_mode: u8,
    pub buckets: Vec<HashBucket>,
}

// `STBDS_SH_*`
pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

// `STBDS_HM_*`
pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;

// ---------------------------------------------------------------------------
// Function-pointer table
// ---------------------------------------------------------------------------

pub type FnArrGrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrFreef = unsafe extern "C" fn(*mut c_void);
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnHmFree = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmGetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type FnHmGetKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmPutKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnShModeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnStrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
pub type FnStrReset = unsafe extern "C" fn(*mut StringArena);
pub type FnStrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnArrDel = unsafe extern "C" fn(c_int);

pub struct Lib {
    pub name: &'static str,
    pub arrgrowf: FnArrGrowf,
    pub arrfreef: FnArrFreef,
    pub rand_seed: FnRandSeed,
    pub hash_bytes: FnHashBytes,
    pub hash_string: FnHashString,
    pub hmfree_func: FnHmFree,
    pub hmget_key_ts: FnHmGetKeyTs,
    pub hmget_key: FnHmGetKey,
    pub hmput_default: FnHmPutDefault,
    pub hmput_key: FnHmPutKey,
    pub hmdel_key: FnHmDelKey,
    pub shmode_func: FnShModeFunc,
    pub stralloc: FnStrAlloc,
    pub strreset: FnStrReset,
    pub strkey: FnStrKey,
    pub arr_del: FnArrDel,
    // Declared last so it is dropped last.
    _lib: Library,
}

unsafe fn get<T: Copy>(lib: &Library, name: &[u8]) -> T {
    unsafe {
        let s: libloading::Symbol<T> = lib
            .get(name)
            .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)));
        *s
    }
}

impl Lib {
    /// Open ONLY the C library (for the single-library subprocess scenarios used
    /// by the abort-parity tests, where the global lock must not be taken).
    pub fn open_c() -> Lib {
        Lib::open("C", &find_c_so())
    }

    /// Open ONLY the Rust library.
    pub fn open_rust() -> Lib {
        Lib::open("Rust", &find_rust_so())
    }

    fn open(name: &'static str, path: &Path) -> Lib {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
            Lib {
                name,
                arrgrowf: get(&lib, b"stbds_arrgrowf\0"),
                arrfreef: get(&lib, b"stbds_arrfreef\0"),
                rand_seed: get(&lib, b"stbds_rand_seed\0"),
                hash_bytes: get(&lib, b"stbds_hash_bytes\0"),
                hash_string: get(&lib, b"stbds_hash_string\0"),
                hmfree_func: get(&lib, b"stbds_hmfree_func\0"),
                hmget_key_ts: get(&lib, b"stbds_hmget_key_ts\0"),
                hmget_key: get(&lib, b"stbds_hmget_key\0"),
                hmput_default: get(&lib, b"stbds_hmput_default\0"),
                hmput_key: get(&lib, b"stbds_hmput_key\0"),
                hmdel_key: get(&lib, b"stbds_hmdel_key\0"),
                shmode_func: get(&lib, b"stbds_shmode_func\0"),
                stralloc: get(&lib, b"stbds_stralloc\0"),
                strreset: get(&lib, b"stbds_strreset\0"),
                strkey: get(&lib, b"strkey\0"),
                arr_del: get(&lib, b"arr_del\0"),
                _lib: lib,
            }
        }
    }
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    let build = crate_root().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found.push(p);
            }
        }
    }
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}; found {found:?}. Build the C lib first:\n  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display()
    );
    found.pop().unwrap()
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("TRANSLATION_SO") {
        return PathBuf::from(p);
    }
    let root = crate_root();
    for prof in ["release", "debug"] {
        let p = root.join("target").join(prof).join("libarr_del_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "no libarr_del_lib.so found under {}/target/{{release,debug}}. Run `cargo build --release` first.",
        root.display()
    );
}

/// Both `.so`s carry a *process-global* `stbds_hash_seed` (and a global `buffer`
/// for `strkey`). Tests therefore must not run concurrently; this lock enforces
/// that even under the default multi-threaded test harness.
static GLOBAL_STATE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
    _guard: std::sync::MutexGuard<'static, ()>,
}

/// Opens both libraries. Each call to `dlopen` on an already-loaded library
/// returns the same handle, so the process-global state (`stbds_hash_seed`,
/// `buffer`) is shared across tests within one test binary. Tests that depend
/// on that state call `reseed()` first.
pub fn pair() -> Pair {
    // Ignore poisoning: a failed test leaves the libraries in whatever state it
    // reached, and every test re-seeds before it does anything meaningful.
    let guard = GLOBAL_STATE_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    Pair {
        c: Lib::open("C", &find_c_so()),
        rs: Lib::open("Rust", &find_rust_so()),
        _guard: guard,
    }
}

impl Pair {
    /// Put both libraries' global `stbds_hash_seed` into the same known state.
    pub fn reseed(&self, seed: usize) {
        unsafe {
            (self.c.rand_seed)(seed);
            (self.rs.rand_seed)(seed);
        }
    }

    /// The default seed the C initialises `stbds_hash_seed` to.
    pub fn reseed_default(&self) {
        self.reseed(0x3141_5926);
    }
}

// ---------------------------------------------------------------------------
// Raw-memory inspection helpers
// ---------------------------------------------------------------------------

/// `stbds_header(a)` — `a` is an *array* pointer (not a hash pointer).
pub unsafe fn header(a: *mut c_void) -> ArrayHeader {
    unsafe { *((a as *mut u8).sub(HDR_SIZE) as *mut ArrayHeader) }
}

/// `STBDS_HASH_TO_ARR`
pub fn hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

/// `STBDS_ARR_TO_HASH`
pub fn arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_add(elemsize) as *mut c_void
}

/// Read the `elemsize * length` bytes of the element array reachable from a
/// *hash* pointer (element 0 is the "default" slot).
pub unsafe fn hash_elems(h: *mut c_void, elemsize: usize) -> Vec<u8> {
    unsafe {
        let a = hash_to_arr(h, elemsize);
        let hdr = header(a);
        std::slice::from_raw_parts(a as *const u8, elemsize * hdr.length).to_vec()
    }
}

/// Read the `elemsize * length` bytes of a plain array pointer.
pub unsafe fn arr_bytes(a: *mut c_void, elemsize: usize) -> Vec<u8> {
    unsafe {
        let hdr = header(a);
        std::slice::from_raw_parts(a as *const u8, elemsize * hdr.length).to_vec()
    }
}

pub unsafe fn table_snapshot(h: *mut c_void, elemsize: usize) -> Option<TableSnapshot> {
    unsafe {
        let a = hash_to_arr(h, elemsize);
        let hdr = header(a);
        if hdr.hash_table.is_null() {
            return None;
        }
        let t = hdr.hash_table as *const HashIndex;
        let ti = *t;
        let nbuckets = ti.slot_count >> BUCKET_SHIFT;
        let buckets = std::slice::from_raw_parts(ti.storage as *const HashBucket, nbuckets).to_vec();
        Some(TableSnapshot {
            slot_count: ti.slot_count,
            used_count: ti.used_count,
            used_count_threshold: ti.used_count_threshold,
            used_count_shrink_threshold: ti.used_count_shrink_threshold,
            tombstone_count: ti.tombstone_count,
            tombstone_count_threshold: ti.tombstone_count_threshold,
            seed: ti.seed,
            slot_count_log2: ti.slot_count_log2,
            arena_has_storage: !ti.string.storage.is_null(),
            arena_remaining: ti.string.remaining,
            arena_block: ti.string.block,
            arena_mode: ti.string.mode,
            buckets,
        })
    }
}

/// Snapshot of an array header that is comparable across libraries (absolute
/// pointers are reduced to "is it null").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HdrSnapshot {
    pub length: usize,
    pub capacity: usize,
    pub has_table: bool,
    pub temp: isize,
}

pub unsafe fn hdr_snapshot(a: *mut c_void) -> HdrSnapshot {
    unsafe {
        let h = header(a);
        HdrSnapshot {
            length: h.length,
            capacity: h.capacity,
            has_table: !h.hash_table.is_null(),
            temp: h.temp,
        }
    }
}

pub unsafe fn hash_hdr_snapshot(h: *mut c_void, elemsize: usize) -> HdrSnapshot {
    unsafe { hdr_snapshot(hash_to_arr(h, elemsize)) }
}

pub unsafe fn cstr(p: *const c_char) -> Vec<u8> {
    unsafe {
        let mut v = Vec::new();
        let mut q = p as *const u8;
        while *q != 0 {
            v.push(*q);
            q = q.add(1);
        }
        v
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xoshiro256**) — no external crate, fixed seed.
// ---------------------------------------------------------------------------

pub struct Rng {
    s: [u64; 4],
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        // SplitMix64 to fill the state.
        let mut x = seed;
        let mut s = [0u64; 4];
        for slot in s.iter_mut() {
            x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = x;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            *slot = z ^ (z >> 31);
        }
        Rng { s }
    }

    pub fn next_u64(&mut self) -> u64 {
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
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

    /// A random NUL-free byte string of length `len`, plus a trailing NUL.
    pub fn cstring(&mut self, len: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..len).map(|_| 1 + (self.next_u64() % 255) as u8).collect();
        v.push(0);
        v
    }
}

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

/// Differential equality assertion with a context label.
#[track_caller]
pub fn dq<T: PartialEq + std::fmt::Debug>(ctx: impl std::fmt::Display, c: T, r: T) {
    assert!(c == r, "divergence [{ctx}]\n  C   : {c:?}\n  Rust: {r:?}");
}

#[macro_export]
macro_rules! diffeq {
    ($ctx:expr, $c:expr, $r:expr) => {{
        let cv = $c;
        let rv = $r;
        assert!(
            cv == rv,
            "divergence [{}]\n  C   : {:?}\n  Rust: {:?}",
            $ctx,
            cv,
            rv
        );
    }};
}

pub mod map;

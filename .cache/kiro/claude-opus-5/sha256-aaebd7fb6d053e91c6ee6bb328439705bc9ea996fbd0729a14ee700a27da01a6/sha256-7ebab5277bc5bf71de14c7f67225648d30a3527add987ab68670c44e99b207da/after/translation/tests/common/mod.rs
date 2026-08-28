//! Shared harness: loads the C `.so` and the Rust `.so` through `libloading`
//! and exposes both behind an identical `Api` façade so every test can drive
//! the two implementations through their exported symbols only.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};

use libloading::Library;

// ---------------------------------------------------------------------------
// Layout mirrors of the C structures (used only to *inspect* results)
// ---------------------------------------------------------------------------

pub const STBDS_BUCKET_LENGTH: usize = 8;
pub const STBDS_HM_BINARY: c_int = 0;
pub const STBDS_HM_STRING: c_int = 1;
pub const STBDS_SH_NONE: c_int = 0;
pub const STBDS_SH_DEFAULT: c_int = 1;
pub const STBDS_SH_STRDUP: c_int = 2;
pub const STBDS_SH_ARENA: c_int = 3;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
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

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HashBucket {
    pub hash: [usize; STBDS_BUCKET_LENGTH],
    pub index: [isize; STBDS_BUCKET_LENGTH],
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

/// `struct { char *key; int value; }` — the element type used by `sh_geti`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct StrMapEntry {
    pub key: *mut c_char,
    pub value: c_int,
}

pub const ELEMSIZE: usize = std::mem::size_of::<StrMapEntry>();
pub const KEYSIZE: usize = std::mem::size_of::<*mut c_char>();

// ---------------------------------------------------------------------------
// Function pointer types
// ---------------------------------------------------------------------------

pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnStralloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
pub type FnStrreset = unsafe extern "C" fn(*mut StringArena);
pub type FnArrgrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrfreef = unsafe extern "C" fn(*mut c_void);
pub type FnHmfreeFunc = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmgetKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmgetKeyTs = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *mut c_void,
    usize,
    *mut isize,
    c_int,
) -> *mut c_void;
pub type FnHmputDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmputKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmdelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnShmodeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnStrkey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnShGeti = unsafe extern "C" fn(c_int);

/// Every exported symbol of one implementation, resolved through `dlsym`.
pub struct Api {
    pub name: &'static str,
    _lib: &'static Library,
    pub rand_seed: FnRandSeed,
    pub hash_bytes: FnHashBytes,
    pub hash_string: FnHashString,
    pub stralloc: FnStralloc,
    pub strreset: FnStrreset,
    pub arrgrowf: FnArrgrowf,
    pub arrfreef: FnArrfreef,
    pub hmfree_func: FnHmfreeFunc,
    pub hmget_key: FnHmgetKey,
    pub hmget_key_ts: FnHmgetKeyTs,
    pub hmput_default: FnHmputDefault,
    pub hmput_key: FnHmputKey,
    pub hmdel_key: FnHmdelKey,
    pub shmode_func: FnShmodeFunc,
    pub strkey: FnStrkey,
    pub sh_geti: FnShGeti,
}

macro_rules! sym {
    ($lib:expr, $ty:ty, $name:literal) => {{
        let s = unsafe { $lib.get::<$ty>($name) }
            .unwrap_or_else(|e| panic!("missing symbol {:?}: {}", $name, e));
        *s
    }};
}

impl Api {
    fn load(name: &'static str, path: &Path) -> Api {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("cannot dlopen {}: {}", path.display(), e));
        let lib: &'static Library = Box::leak(Box::new(lib));
        Api {
            name,
            rand_seed: sym!(lib, FnRandSeed, b"stbds_rand_seed\0"),
            hash_bytes: sym!(lib, FnHashBytes, b"stbds_hash_bytes\0"),
            hash_string: sym!(lib, FnHashString, b"stbds_hash_string\0"),
            stralloc: sym!(lib, FnStralloc, b"stbds_stralloc\0"),
            strreset: sym!(lib, FnStrreset, b"stbds_strreset\0"),
            arrgrowf: sym!(lib, FnArrgrowf, b"stbds_arrgrowf\0"),
            arrfreef: sym!(lib, FnArrfreef, b"stbds_arrfreef\0"),
            hmfree_func: sym!(lib, FnHmfreeFunc, b"stbds_hmfree_func\0"),
            hmget_key: sym!(lib, FnHmgetKey, b"stbds_hmget_key\0"),
            hmget_key_ts: sym!(lib, FnHmgetKeyTs, b"stbds_hmget_key_ts\0"),
            hmput_default: sym!(lib, FnHmputDefault, b"stbds_hmput_default\0"),
            hmput_key: sym!(lib, FnHmputKey, b"stbds_hmput_key\0"),
            hmdel_key: sym!(lib, FnHmdelKey, b"stbds_hmdel_key\0"),
            shmode_func: sym!(lib, FnShmodeFunc, b"stbds_shmode_func\0"),
            strkey: sym!(lib, FnStrkey, b"strkey\0"),
            sh_geti: sym!(lib, FnShGeti, b"sh_geti\0"),
            _lib: lib,
        }
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().unwrap().to_path_buf()
}

fn c_so_path() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut found = None;
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let n = p.file_name().unwrap().to_string_lossy().to_string();
            if n.starts_with("lib") && n.ends_with(".so") {
                found = Some(p);
                break;
            }
        }
    }
    found.unwrap_or_else(|| {
        panic!(
            "no C shared library found in {} — build it with cmake first",
            build.display()
        )
    })
}

fn rust_so_path() -> PathBuf {
    // current_exe is <target-dir>/<profile>/deps/<test>-<hash>
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe.parent().unwrap().parent().unwrap();
    let so = profile_dir.join("libsh_geti_lib.so");

    // No fallback to another profile directory on purpose: silently loading a
    // different build than the one under test is exactly the failure mode this
    // guard exists to prevent.
    assert!(
        so.exists(),
        "{} does not exist. `cargo test` does not build a `cdylib` lib target — \
         run `cargo build` (or ./run_tests.sh) first.",
        so.display()
    );

    // Likewise, the .so next to the test binaries can silently be stale.
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    if let (Ok(mso), Ok(msrc)) = (
        std::fs::metadata(&so).and_then(|m| m.modified()),
        std::fs::metadata(&src).and_then(|m| m.modified()),
    ) {
        assert!(
            mso >= msrc,
            "{} is older than {} — run `cargo build` (or ./run_tests.sh) before `cargo test`; \
             `cargo test` does not rebuild a cdylib target",
            so.display(),
            src.display()
        );
    }
    so
}

/// Loads both libraries. Each call performs a fresh `dlopen`, but the dynamic
/// loader caches by path so the *same* copy (and therefore the same mutable
/// `stbds_hash_seed`) is shared inside one test process. Tests must therefore
/// reset the seed explicitly through `stbds_rand_seed` when they care.
pub fn apis() -> (Api, Api) {
    let c = Api::load("C", &c_so_path());
    let r = Api::load("Rust", &rust_so_path());
    (c, r)
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

pub const DEFAULT_SEED: usize = 0x31415926;

/// Reset the mutable hash seed of both libraries to the same value.
pub fn reset_seeds(c: &Api, r: &Api, seed: usize) {
    unsafe {
        (c.rand_seed)(seed);
        (r.rand_seed)(seed);
    }
}

pub unsafe fn header(a: *mut c_void) -> ArrayHeader {
    unsafe { *(a as *mut ArrayHeader).sub(1) }
}

pub unsafe fn cstr(p: *const c_char) -> Vec<u8> {
    unsafe {
        let mut v = Vec::new();
        let mut i = 0;
        while *p.add(i) != 0 {
            v.push(*p.add(i) as u8);
            i += 1;
        }
        v
    }
}

/// A NUL-terminated owned C string usable as `*mut c_char`.
pub struct CStr8(pub Vec<u8>);

impl CStr8 {
    pub fn new(s: &str) -> CStr8 {
        let mut v = s.as_bytes().to_vec();
        v.push(0);
        CStr8(v)
    }
    pub fn from_bytes(b: &[u8]) -> CStr8 {
        let mut v = b.to_vec();
        v.push(0);
        CStr8(v)
    }
    pub fn as_ptr(&mut self) -> *mut c_char {
        self.0.as_mut_ptr() as *mut c_char
    }
}

/// Snapshot of a hash index that excludes raw addresses so the two
/// implementations can be compared field by field.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct HashIndexSnapshot {
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
    pub arena_storage_null: bool,
    pub buckets: Vec<(Vec<usize>, Vec<isize>)>,
    pub storage_aligned: bool,
}

pub unsafe fn snapshot_index(t: *mut HashIndex) -> Option<HashIndexSnapshot> {
    unsafe {
        if t.is_null() {
            return None;
        }
        let ti = &*t;
        let nbuckets = ti.slot_count >> 3;
        let mut buckets = Vec::with_capacity(nbuckets);
        for i in 0..nbuckets {
            let b = &*ti.storage.add(i);
            buckets.push((b.hash.to_vec(), b.index.to_vec()));
        }
        Some(HashIndexSnapshot {
            slot_count: ti.slot_count,
            used_count: ti.used_count,
            used_count_threshold: ti.used_count_threshold,
            used_count_shrink_threshold: ti.used_count_shrink_threshold,
            tombstone_count: ti.tombstone_count,
            tombstone_count_threshold: ti.tombstone_count_threshold,
            seed: ti.seed,
            slot_count_log2: ti.slot_count_log2,
            arena_remaining: ti.string.remaining,
            arena_block: ti.string.block,
            arena_mode: ti.string.mode,
            arena_storage_null: ti.string.storage.is_null(),
            buckets,
            storage_aligned: (ti.storage as usize) % 64 == 0,
        })
    }
}

/// Snapshot of an `stbds` hash-map array: header fields (minus addresses),
/// element values and the *contents* of the key strings.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MapSnapshot {
    pub null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub table: Option<HashIndexSnapshot>,
    /// `(key contents or None for NULL, value)` for every slot incl. index 0.
    pub entries: Vec<(Option<Vec<u8>>, c_int)>,
}

/// `t` is the hash-map pointer as seen by user code (already offset by one
/// element), i.e. what `stbds_hmput_key` returns.
pub unsafe fn snapshot_map(t: *mut StrMapEntry) -> MapSnapshot {
    unsafe {
        if t.is_null() {
            return MapSnapshot {
                null: true,
                length: 0,
                capacity: 0,
                temp: 0,
                table: None,
                entries: Vec::new(),
            };
        }
        let raw = t.sub(1) as *mut c_void;
        let h = header(raw);
        let table = h.hash_table as *mut HashIndex;
        let snap = snapshot_index(table);
        let mut entries = Vec::new();
        let base = t.sub(1);
        for i in 0..h.length {
            let e = &*base.add(i);
            let k = if e.key.is_null() {
                None
            } else {
                Some(cstr(e.key))
            };
            entries.push((k, e.value));
        }
        MapSnapshot {
            null: false,
            length: h.length,
            capacity: h.capacity,
            temp: h.temp,
            table: snap,
            entries,
        }
    }
}

/// `table->temp_key` contents (the string it points at), if any. Only
/// meaningful after an operation that is documented to set it.
pub unsafe fn temp_key(t: *mut StrMapEntry) -> Option<Vec<u8>> {
    unsafe {
        if t.is_null() {
            return None;
        }
        let h = header(t.sub(1) as *mut c_void);
        let table = h.hash_table as *mut HashIndex;
        if table.is_null() || (*table).temp_key.is_null() {
            None
        } else {
            Some(cstr((*table).temp_key))
        }
    }
}

// ---------------------------------------------------------------------------
// Serialisation
// ---------------------------------------------------------------------------
//
// Both libraries keep a mutable global (`stbds_hash_seed`) and a shared
// `strkey` scratch buffer, and `dlopen` returns the *same* mapping for a given
// path, so tests inside one binary must not run concurrently.

use std::sync::{Mutex, MutexGuard, OnceLock};

static LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub fn serial() -> MutexGuard<'static, ()> {
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

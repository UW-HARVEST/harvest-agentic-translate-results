//! Shared harness: loads the C `.so` and the Rust `.so` through `libloading`
//! and exposes the exported stb_ds API of both so results can be compared.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Mirror of the C structures (needed to inspect the opaque results)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct StringArena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct HashBucket {
    pub hash: [usize; 8],
    pub index: [isize; 8],
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

pub const HDR: usize = std::mem::size_of::<ArrayHeader>();

// ---------------------------------------------------------------------------
// Exported function signatures
// ---------------------------------------------------------------------------

type FnArrgrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type FnArrfreef = unsafe extern "C" fn(*mut c_void);
type FnRandSeed = unsafe extern "C" fn(usize);
type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
type FnHmfreeFunc = unsafe extern "C" fn(*mut c_void, usize);
type FnHmgetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
type FnHmgetKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type FnHmputDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FnHmputKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type FnShmodeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
type FnHmdelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
type FnStralloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
type FnStrreset = unsafe extern "C" fn(*mut StringArena);
type FnHmGeti = unsafe extern "C" fn(c_int);
type FnStrkey = unsafe extern "C" fn(c_int) -> *mut c_char;

/// One loaded implementation (either the C build or the Rust build).
pub struct Impl {
    _lib: Library,
    pub name: &'static str,
    pub arrgrowf: FnArrgrowf,
    pub arrfreef: FnArrfreef,
    pub rand_seed: FnRandSeed,
    pub hash_bytes: FnHashBytes,
    pub hash_string: FnHashString,
    pub hmfree_func: FnHmfreeFunc,
    pub hmget_key_ts: FnHmgetKeyTs,
    pub hmget_key: FnHmgetKey,
    pub hmput_default: FnHmputDefault,
    pub hmput_key: FnHmputKey,
    pub shmode_func: FnShmodeFunc,
    pub hmdel_key: FnHmdelKey,
    pub stralloc: FnStralloc,
    pub strreset: FnStrreset,
    pub hm_geti: FnHmGeti,
    pub strkey: FnStrkey,
}

unsafe fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
    unsafe {
        let s: Symbol<T> = lib
            .get(name)
            .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)));
        *s
    }
}

impl Impl {
    pub fn load(path: &Path, name: &'static str) -> Impl {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("cannot load {}: {e}", path.display()));
            Impl {
                arrgrowf: sym(&lib, b"stbds_arrgrowf\0"),
                arrfreef: sym(&lib, b"stbds_arrfreef\0"),
                rand_seed: sym(&lib, b"stbds_rand_seed\0"),
                hash_bytes: sym(&lib, b"stbds_hash_bytes\0"),
                hash_string: sym(&lib, b"stbds_hash_string\0"),
                hmfree_func: sym(&lib, b"stbds_hmfree_func\0"),
                hmget_key_ts: sym(&lib, b"stbds_hmget_key_ts\0"),
                hmget_key: sym(&lib, b"stbds_hmget_key\0"),
                hmput_default: sym(&lib, b"stbds_hmput_default\0"),
                hmput_key: sym(&lib, b"stbds_hmput_key\0"),
                shmode_func: sym(&lib, b"stbds_shmode_func\0"),
                hmdel_key: sym(&lib, b"stbds_hmdel_key\0"),
                stralloc: sym(&lib, b"stbds_stralloc\0"),
                strreset: sym(&lib, b"stbds_strreset\0"),
                hm_geti: sym(&lib, b"hm_geti\0"),
                strkey: sym(&lib, b"strkey\0"),
                name,
                _lib: lib,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let dir = workspace_root().join("c_src").join("build");
    let mut best: Option<PathBuf> = None;
    let entries = std::fs::read_dir(&dir).unwrap_or_else(|e| {
        panic!(
            "cannot read {} ({e}); build the C library first:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            dir.display()
        )
    });
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().map(|x| x == "so").unwrap_or(false) {
            best = Some(p);
        }
    }
    best.unwrap_or_else(|| panic!("no .so found in {}", dir.display()))
}

fn find_rust_so() -> PathBuf {
    // allow pointing the suite at a specific build (e.g. the release cdylib)
    if let Ok(p) = std::env::var("RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "RUST_SO={} is not a file", p.display());
        return p;
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut candidates: Vec<PathBuf> = Vec::new();

    // target/<profile>/ derived from the running test binary (target/<p>/deps/<exe>)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(profile_dir) = exe.parent().and_then(|p| p.parent()) {
            candidates.push(profile_dir.join("libhm_geti_lib.so"));
        }
    }
    candidates.push(manifest.join("target/release/libhm_geti_lib.so"));
    candidates.push(manifest.join("target/debug/libhm_geti_lib.so"));

    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib not found (looked in {:?}); run `cargo build` / `cargo build --release` first",
        candidates
    );
}

pub fn c_so_path() -> PathBuf {
    find_c_so()
}

pub fn rust_so_path() -> PathBuf {
    find_rust_so()
}

/// The pair under test.
///
/// Both libraries keep a *mutable global* `stbds_hash_seed`, and `dlopen` of the
/// same path from several tests returns the same (reference-counted) handle, so
/// that state is shared process-wide. `load_pair` therefore serialises tests:
/// only one may drive the libraries at a time, otherwise the seed sequences of
/// the C and Rust sides interleave differently and diverge.
pub struct Pair {
    pub c: Impl,
    pub r: Impl,
    _guard: std::sync::MutexGuard<'static, ()>,
}

static SEED_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn load_pair() -> Pair {
    let guard = SEED_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    Pair {
        c: Impl::load(&find_c_so(), "C"),
        r: Impl::load(&find_rust_so(), "Rust"),
        _guard: guard,
    }
}

impl Pair {
    /// Put both libraries' `stbds_hash_seed` globals into the same state so
    /// that hashing (and therefore probe order) is deterministic and matched.
    pub fn reset_seed(&self, seed: usize) {
        unsafe {
            (self.c.rand_seed)(seed);
            (self.r.rand_seed)(seed);
        }
    }
}

pub const DEFAULT_SEED: usize = 0x31415926;

// ---------------------------------------------------------------------------
// Snapshotting an stb_ds hash-array so the two sides can be byte-compared
// ---------------------------------------------------------------------------

/// Header of a *raw array* pointer (`a`, as returned by `stbds_arrgrowf`).
pub unsafe fn header(a: *mut c_void) -> ArrayHeader {
    unsafe { *((a as *mut u8).sub(HDR) as *mut ArrayHeader) }
}

fn push_usize(out: &mut Vec<u8>, v: usize) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn push_isize(out: &mut Vec<u8>, v: isize) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// Serialise everything about an array header that must match, i.e. everything
/// except the raw `hash_table` address (only its null-ness is meaningful).
pub unsafe fn snapshot_header(out: &mut Vec<u8>, a: *mut c_void) {
    unsafe {
        let h = header(a);
        push_usize(out, h.length);
        push_usize(out, h.capacity);
        push_isize(out, h.temp);
        out.push(h.hash_table.is_null() as u8);
    }
}

/// Serialise the hash index (skipping the three address-valued fields).
pub unsafe fn snapshot_hash_index(out: &mut Vec<u8>, a: *mut c_void) {
    unsafe {
        let h = header(a);
        if h.hash_table.is_null() {
            out.push(0);
            return;
        }
        out.push(1);
        let t = h.hash_table as *mut HashIndex;
        push_usize(out, (*t).slot_count);
        push_usize(out, (*t).used_count);
        push_usize(out, (*t).used_count_threshold);
        push_usize(out, (*t).used_count_shrink_threshold);
        push_usize(out, (*t).tombstone_count);
        push_usize(out, (*t).tombstone_count_threshold);
        push_usize(out, (*t).seed);
        push_usize(out, (*t).slot_count_log2);
        push_usize(out, (*t).string.remaining);
        out.push((*t).string.block);
        out.push((*t).string.mode);
        out.push((*t).string.storage.is_null() as u8);
        // NOTE: `temp_key` is deliberately *not* compared here. Neither
        // implementation initialises it in stbds_make_hash_index (the block
        // comes straight from realloc), so it holds indeterminate bytes until a
        // string-mode insert writes it. The string-map tests check it directly.

        let nbuckets = (*t).slot_count >> 3;
        for i in 0..nbuckets {
            let b = (*t).storage.add(i);
            for j in 0..8 {
                push_usize(out, (*b).hash[j]);
            }
            for j in 0..8 {
                push_isize(out, (*b).index[j]);
            }
        }
    }
}

/// Full snapshot of a hash-map handle `t` (the value the `stbds_hm*` functions
/// return, which points at element 1 of the raw array).
///
/// `plain_elems` should only be set when the element type contains no padding
/// and no pointers, so that a raw byte compare is meaningful.
pub unsafe fn snapshot_map(t: *mut c_void, elemsize: usize, plain_elems: bool) -> Vec<u8> {
    unsafe {
        let mut out = Vec::new();
        if t.is_null() {
            out.push(0);
            return out;
        }
        out.push(1);
        let a = (t as *mut u8).sub(elemsize) as *mut c_void;
        snapshot_header(&mut out, a);
        snapshot_hash_index(&mut out, a);
        if plain_elems {
            let len = header(a).length;
            let bytes = std::slice::from_raw_parts(a as *const u8, len * elemsize);
            out.extend_from_slice(bytes);
        }
        out
    }
}

pub fn assert_bytes_eq(what: &str, c: &[u8], r: &[u8]) {
    if c != r {
        let n = c.len().min(r.len());
        let mut first = None;
        for i in 0..n {
            if c[i] != r[i] {
                first = Some(i);
                break;
            }
        }
        panic!(
            "{what}: C/Rust snapshots differ (C {} bytes, Rust {} bytes, first diff at {:?})\n\
             C   = {:02x?}\nRust= {:02x?}",
            c.len(),
            r.len(),
            first,
            &c[..n.min(256)],
            &r[..n.min(256)]
        );
    }
}

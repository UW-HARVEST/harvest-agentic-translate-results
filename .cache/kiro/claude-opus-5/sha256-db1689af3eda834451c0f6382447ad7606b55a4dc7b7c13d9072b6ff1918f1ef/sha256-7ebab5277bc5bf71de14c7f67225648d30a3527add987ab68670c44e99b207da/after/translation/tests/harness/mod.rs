//! Shared FFI harness: loads BOTH the C reference `.so` and the Rust `.so`
//! through `libloading` and exposes identically-shaped wrappers so every test
//! can drive the two implementations through their exported symbols only.
#![allow(dead_code)]
#![allow(non_camel_case_types)]

use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Layout mirrors of the C structs (used only to *inspect* results).
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct StringArena {
    pub storage: *mut StringBlock,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

impl StringArena {
    pub fn zeroed() -> Self {
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
pub struct StringBlock {
    pub next: *mut StringBlock,
    pub storage: [c_char; 8],
}

pub const BUCKET_LENGTH: usize = 8;
pub const BUCKET_SHIFT: usize = 3;
pub const STBDS_CACHE_LINE_SIZE: usize = 64;

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

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;

pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

/// The element type used by `str_dups`: `struct { char *key; int value; }`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct StrEntry {
    pub key: *mut c_char,
    pub value: c_int,
}

/// A binary-keyed element: `struct { int key; int value; }`.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct IntEntry {
    pub key: c_int,
    pub value: c_int,
}

// ---------------------------------------------------------------------------
// Locating / building the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    manifest_dir().parent().unwrap().to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut found: Option<PathBuf> = None;
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                found = Some(p);
                break;
            }
        }
    }
    found.unwrap_or_else(|| {
        panic!(
            "C shared library not found under {}; build it with cmake first",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    // An explicit override lets the same tests be run against either the
    // debug or the release cdylib (they differ in optimisation level and in
    // `panic = "abort"`, so both are worth exercising).
    if let Ok(p) = std::env::var("STR_DUPS_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "STR_DUPS_RUST_SO={} does not exist", p.display());
        return p;
    }
    // Otherwise prefer the freshest of release/debug.
    let mut cands: Vec<PathBuf> = Vec::new();
    for profile in ["release", "debug"] {
        let p = manifest_dir()
            .join("target")
            .join(profile)
            .join("libstr_dups_lib.so");
        if p.exists() {
            cands.push(p);
        }
    }
    if cands.is_empty() {
        // Build it.
        let st = std::process::Command::new(env!("CARGO"))
            .args(["build", "--release"])
            .current_dir(manifest_dir())
            .status()
            .expect("failed to spawn cargo build");
        assert!(st.success(), "cargo build --release failed");
        let p = manifest_dir().join("target/release/libstr_dups_lib.so");
        assert!(p.exists(), "cdylib missing after build");
        return p;
    }
    cands.sort_by_key(|p| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });
    cands.pop().unwrap()
}

// ---------------------------------------------------------------------------
// Symbol table
// ---------------------------------------------------------------------------

pub type FnArrgrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrfreef = unsafe extern "C" fn(*mut c_void);
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnStralloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
pub type FnStrreset = unsafe extern "C" fn(*mut StringArena);
pub type FnHmfreeFunc = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmgetKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmgetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type FnHmputDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmputKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmdelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnShmodeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnStrDups = unsafe extern "C" fn(c_int);
pub type FnStrkey = unsafe extern "C" fn(c_int) -> *mut c_char;

pub struct Impl {
    pub name: &'static str,
    _lib: libloading::Library,
    pub arrgrowf: FnArrgrowf,
    pub arrfreef: FnArrfreef,
    pub rand_seed: FnRandSeed,
    pub hash_bytes: FnHashBytes,
    pub hash_string: FnHashString,
    pub stralloc: FnStralloc,
    pub strreset: FnStrreset,
    pub hmfree_func: FnHmfreeFunc,
    pub hmget_key: FnHmgetKey,
    pub hmget_key_ts: FnHmgetKeyTs,
    pub hmput_default: FnHmputDefault,
    pub hmput_key: FnHmputKey,
    pub hmdel_key: FnHmdelKey,
    pub shmode_func: FnShmodeFunc,
    pub str_dups: FnStrDups,
    pub strkey: FnStrkey,
}

macro_rules! sym {
    ($lib:expr, $ty:ty, $n:literal) => {{
        let s: libloading::Symbol<$ty> = unsafe { $lib.get($n).expect(concat!("missing ", stringify!($n))) };
        *s
    }};
}

impl Impl {
    fn load(name: &'static str, path: &Path) -> Impl {
        let lib = unsafe { libloading::Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
        let me = Impl {
            name,
            arrgrowf: sym!(lib, FnArrgrowf, b"stbds_arrgrowf\0"),
            arrfreef: sym!(lib, FnArrfreef, b"stbds_arrfreef\0"),
            rand_seed: sym!(lib, FnRandSeed, b"stbds_rand_seed\0"),
            hash_bytes: sym!(lib, FnHashBytes, b"stbds_hash_bytes\0"),
            hash_string: sym!(lib, FnHashString, b"stbds_hash_string\0"),
            stralloc: sym!(lib, FnStralloc, b"stbds_stralloc\0"),
            strreset: sym!(lib, FnStrreset, b"stbds_strreset\0"),
            hmfree_func: sym!(lib, FnHmfreeFunc, b"stbds_hmfree_func\0"),
            hmget_key: sym!(lib, FnHmgetKey, b"stbds_hmget_key\0"),
            hmget_key_ts: sym!(lib, FnHmgetKeyTs, b"stbds_hmget_key_ts\0"),
            hmput_default: sym!(lib, FnHmputDefault, b"stbds_hmput_default\0"),
            hmput_key: sym!(lib, FnHmputKey, b"stbds_hmput_key\0"),
            hmdel_key: sym!(lib, FnHmdelKey, b"stbds_hmdel_key\0"),
            shmode_func: sym!(lib, FnShmodeFunc, b"stbds_shmode_func\0"),
            str_dups: sym!(lib, FnStrDups, b"str_dups\0"),
            strkey: sym!(lib, FnStrkey, b"strkey\0"),
            _lib: lib,
        };
        me
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

static PAIR: OnceLock<Pair> = OnceLock::new();

/// Both `.so`s carry process-global state (`stbds_hash_seed`, the `strkey`
/// static buffer) and `capture_stdout` rebinds fd 1 for the whole process, so
/// any test that touches either must run serially.
static GLOBAL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn global_lock() -> std::sync::MutexGuard<'static, ()> {
    GLOBAL_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: Impl::load("C", &find_c_so()),
        rs: Impl::load("Rust", &find_rust_so()),
    })
}

// ---------------------------------------------------------------------------
// Inspection helpers — everything below returns allocation-independent,
// directly comparable values.
// ---------------------------------------------------------------------------

pub unsafe fn header(arr: *mut c_void) -> ArrayHeader {
    unsafe { *(arr as *mut ArrayHeader).offset(-1) }
}

/// `(length, capacity, temp, hash_table_is_null)`
pub unsafe fn header_snap(arr: *mut c_void) -> (usize, usize, isize, bool) {
    let h = unsafe { header(arr) };
    (h.length, h.capacity, h.temp, h.hash_table.is_null())
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct IndexSnap {
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
    pub arena_blocks: usize,
    /// Every bucket's hash / index words, in slot order.
    pub buckets: Vec<(usize, isize)>,
    /// Byte offset of `storage` past the end of the `HashIndex` struct.
    /// Allocator-dependent; validated, never cross-compared.
    pub storage_align: usize,
}

pub unsafe fn arena_block_count(a: *const StringArena) -> usize {
    unsafe {
        let mut n = 0usize;
        let mut x = (*a).storage;
        while !x.is_null() {
            n += 1;
            x = (*x).next;
            if n > 1_000_000 {
                break;
            }
        }
        n
    }
}

pub unsafe fn index_snap(t: *mut HashIndex) -> Option<IndexSnap> {
    unsafe {
        if t.is_null() {
            return None;
        }
        let ti = *t;
        // Per-implementation invariant from STBDS_ALIGN_FWD: the bucket array
        // is cache-line aligned and sits within the trailing 63 bytes of slack
        // that `stbds_make_hash_index` allocated for it.
        let align_pad = (ti.storage as usize)
            .wrapping_sub(t as usize)
            .wrapping_sub(std::mem::size_of::<HashIndex>());
        assert_eq!(
            (ti.storage as usize) % STBDS_CACHE_LINE_SIZE,
            0,
            "bucket storage is not cache-line aligned"
        );
        assert!(
            align_pad < STBDS_CACHE_LINE_SIZE,
            "bucket storage padding {align_pad} exceeds the allocated slack"
        );
        let mut buckets = Vec::new();
        let nb = ti.slot_count >> BUCKET_SHIFT;
        for i in 0..nb {
            let b = *ti.storage.add(i);
            for j in 0..BUCKET_LENGTH {
                buckets.push((b.hash[j], b.index[j]));
            }
        }
        Some(IndexSnap {
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
            arena_blocks: arena_block_count(&raw const ti.string),
            buckets,
            storage_align: (ti.storage as usize)
                .wrapping_sub(t as usize)
                .wrapping_sub(std::mem::size_of::<HashIndex>()),
        })
    }
}

/// Full snapshot of a string-keyed map, addressed by its *hash* pointer.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct StrMapSnap {
    pub header: (usize, usize, isize, bool),
    pub index: Option<IndexSnap>,
    /// element 0 is the default/sentinel slot
    pub entries: Vec<(Option<Vec<u8>>, c_int)>,
}

pub unsafe fn cstr(p: *const c_char) -> Option<Vec<u8>> {
    unsafe {
        if p.is_null() {
            None
        } else {
            Some(std::ffi::CStr::from_ptr(p).to_bytes().to_vec())
        }
    }
}

pub unsafe fn str_map_snap(hashp: *mut c_void) -> StrMapSnap {
    unsafe {
        let raw = (hashp as *mut u8).sub(std::mem::size_of::<StrEntry>()) as *mut c_void;
        let h = header(raw);
        let idx = index_snap(h.hash_table as *mut HashIndex);
        let base = hashp as *mut StrEntry;
        let mut entries = Vec::new();
        // element -1 (== raw[0]) is the sentinel; report indices 0..len-1 of raw
        for i in 0..h.length {
            let e = *(raw as *mut StrEntry).add(i);
            entries.push((cstr(e.key), e.value));
        }
        let _ = base;
        // NOTE: `stbds_temp_key` (== *(char**)hash_table) is intentionally NOT
        // part of this snapshot. `stbds_make_hash_index` never initialises
        // `temp_key`, so it holds realloc garbage until an insert writes it —
        // reading it unconditionally would be undefined. Tests compare it
        // explicitly right after an insert instead.
        StrMapSnap {
            header: (h.length, h.capacity, h.temp, h.hash_table.is_null()),
            index: idx,
            entries,
        }
    }
}

/// Full snapshot of a binary-keyed `IntEntry` map, addressed by its hash pointer.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct IntMapSnap {
    pub header: (usize, usize, isize, bool),
    pub index: Option<IndexSnap>,
    pub entries: Vec<IntEntry>,
}

pub unsafe fn int_map_snap(hashp: *mut c_void) -> IntMapSnap {
    unsafe {
        let raw = (hashp as *mut u8).sub(std::mem::size_of::<IntEntry>()) as *mut c_void;
        let h = header(raw);
        let idx = index_snap(h.hash_table as *mut HashIndex);
        let mut entries = Vec::new();
        for i in 0..h.length {
            entries.push(*(raw as *mut IntEntry).add(i));
        }
        IntMapSnap {
            header: (h.length, h.capacity, h.temp, h.hash_table.is_null()),
            index: idx,
            entries,
        }
    }
}

// ---------------------------------------------------------------------------
// stdout capture (for `str_dups`, which prints through libc `printf`)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Runs `f` with fd 1 redirected into a temporary file and returns the bytes
/// written. libc `stdout` is shared by both `.so`s, so this captures either.
pub fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let path = std::env::temp_dir().join(format!(
        "strdups-capture-{}-{}-{:?}.txt",
        std::process::id(),
        tag,
        std::thread::current().id()
    ));
    let file = std::fs::File::create(&path).expect("create capture file");
    use std::os::unix::io::AsRawFd;
    let fd = file.as_raw_fd();
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0);
        assert!(dup2(fd, 1) >= 0);
        f();
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0);
        close(saved);
    }
    drop(file);
    let out = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    out
}

// ---------------------------------------------------------------------------
// Concise diffing (bucket vectors are large; never dump them wholesale)
// ---------------------------------------------------------------------------

fn diff_index(c: &Option<IndexSnap>, r: &Option<IndexSnap>) -> Option<String> {
    match (c, r) {
        (None, None) => None,
        (Some(_), None) => Some("C has a hash index, Rust does not".into()),
        (None, Some(_)) => Some("Rust has a hash index, C does not".into()),
        (Some(a), Some(b)) => {
            macro_rules! f {
                ($($n:ident),*) => {
                    $(if a.$n != b.$n {
                        return Some(format!("index.{}: C={:?} Rust={:?}", stringify!($n), a.$n, b.$n));
                    })*
                };
            }
            f!(
                slot_count,
                used_count,
                used_count_threshold,
                used_count_shrink_threshold,
                tombstone_count,
                tombstone_count_threshold,
                seed,
                slot_count_log2,
                arena_remaining,
                arena_block,
                arena_mode,
                arena_blocks
            );
            // NOTE: `storage_align` is deliberately *not* cross-compared: it
            // is the padding between the malloc'd base and the 64-byte
            // aligned bucket array, so it depends on the address malloc
            // happened to return. Validity is checked per-snapshot instead.
            if a.buckets.len() != b.buckets.len() {
                return Some(format!(
                    "index.buckets len: C={} Rust={}",
                    a.buckets.len(),
                    b.buckets.len()
                ));
            }
            for i in 0..a.buckets.len() {
                if a.buckets[i] != b.buckets[i] {
                    return Some(format!(
                        "index.buckets[{i}] (slot {i}): C=(hash {:#x}, index {}) Rust=(hash {:#x}, index {})",
                        a.buckets[i].0, a.buckets[i].1, b.buckets[i].0, b.buckets[i].1
                    ));
                }
            }
            None
        }
    }
}

pub fn diff_int_map(c: &IntMapSnap, r: &IntMapSnap) -> Option<String> {
    if c.header != r.header {
        return Some(format!(
            "header (len, cap, temp, table_null): C={:?} Rust={:?}",
            c.header, r.header
        ));
    }
    if let Some(d) = diff_index(&c.index, &r.index) {
        return Some(d);
    }
    if c.entries.len() != r.entries.len() {
        return Some(format!(
            "entry count: C={} Rust={}",
            c.entries.len(),
            r.entries.len()
        ));
    }
    for i in 0..c.entries.len() {
        if c.entries[i] != r.entries[i] {
            return Some(format!(
                "entries[{i}]: C={:?} Rust={:?}",
                c.entries[i], r.entries[i]
            ));
        }
    }
    None
}

pub fn diff_str_map(c: &StrMapSnap, r: &StrMapSnap) -> Option<String> {
    if c.header != r.header {
        return Some(format!(
            "header (len, cap, temp, table_null): C={:?} Rust={:?}",
            c.header, r.header
        ));
    }
    if let Some(d) = diff_index(&c.index, &r.index) {
        return Some(d);
    }
    if c.entries.len() != r.entries.len() {
        return Some(format!(
            "entry count: C={} Rust={}",
            c.entries.len(),
            r.entries.len()
        ));
    }
    for i in 0..c.entries.len() {
        if c.entries[i] != r.entries[i] {
            return Some(format!(
                "entries[{i}]: C=({:?}, {}) Rust=({:?}, {})",
                c.entries[i].0.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
                c.entries[i].1,
                r.entries[i].0.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
                r.entries[i].1
            ));
        }
    }
    None
}

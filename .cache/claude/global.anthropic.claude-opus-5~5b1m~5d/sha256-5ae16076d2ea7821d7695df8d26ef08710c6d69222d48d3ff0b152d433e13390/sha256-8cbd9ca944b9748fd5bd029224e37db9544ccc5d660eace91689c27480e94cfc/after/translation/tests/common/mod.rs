//! Shared differential-test harness.
//!
//! BOTH libraries are loaded as shared objects through `libloading` and every
//! call goes through the `.so` export table — the Rust crate is never called
//! directly, so the `#[no_mangle] extern "C"` wrappers are part of what is
//! under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

/// Both libraries keep the hash seed (`stbds_hash_seed`) in a **process-global**
/// mutable, and `dlopen` of the same path from several threads yields the same
/// loaded object. Tests must therefore not run concurrently, or one test's
/// `stbds_make_hash_index` will advance the seed underneath another's. Every
/// `both()` acquires this lock and holds it until the returned `Lib`s drop.
static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    // A panicking test poisons the mutex; recover so the remaining tests still
    // report their real result instead of a poison error.
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// Layout mirrors of the C structs (used only to *read* state for comparison)
// ---------------------------------------------------------------------------

pub const HEADER_SIZE: usize = 32;
pub const BUCKET_LEN: usize = 8;
pub const BUCKET_SHIFT: usize = 3;

// stbds_array_header field offsets, relative to the header start
const OFF_LENGTH: usize = 0;
const OFF_CAPACITY: usize = 8;
const OFF_HASH_TABLE: usize = 16;
const OFF_TEMP: usize = 24;

// stbds_hash_index field offsets
const HI_TEMP_KEY: usize = 0;
const HI_SLOT_COUNT: usize = 8;
const HI_USED_COUNT: usize = 16;
const HI_USED_COUNT_THRESHOLD: usize = 24;
const HI_USED_COUNT_SHRINK_THRESHOLD: usize = 32;
const HI_TOMBSTONE_COUNT: usize = 40;
const HI_TOMBSTONE_COUNT_THRESHOLD: usize = 48;
const HI_SEED: usize = 56;
const HI_SLOT_COUNT_LOG2: usize = 64;
const HI_STRING: usize = 72; // stbds_string_arena
const HI_STORAGE: usize = 96;

// stbds_string_arena field offsets, relative to the arena start
pub const SA_STORAGE: usize = 0;
pub const SA_REMAINING: usize = 8;
pub const SA_BLOCK: usize = 16;
pub const SA_MODE: usize = 17;
pub const SA_SIZE: usize = 24;

/// `struct stbds_string_arena` — 24 bytes, same layout as the C original.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct StringArena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
    pub _pad: [u8; 6],
}

impl StringArena {
    pub fn new() -> Self {
        StringArena {
            storage: std::ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
            _pad: [0; 6],
        }
    }
}

pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;
pub const HM_PTR_TO_STRING: c_int = 2;

// ---------------------------------------------------------------------------
// The loaded library
// ---------------------------------------------------------------------------

type FnArrPush = unsafe extern "C" fn(c_int);
type FnStrkey = unsafe extern "C" fn(c_int) -> *mut c_char;
type FnRandSeed = unsafe extern "C" fn(usize);
type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
type FnArrGrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type FnArrFreef = unsafe extern "C" fn(*mut c_void);
type FnHmFree = unsafe extern "C" fn(*mut c_void, usize);
type FnHmGetKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type FnHmGetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
type FnHmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FnHmPutKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type FnHmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
type FnShModeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
type FnStrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
type FnStrReset = unsafe extern "C" fn(*mut StringArena);

/// All 16 exported symbols, resolved from one `.so`.
pub struct Lib {
    pub name: &'static str,
    /// held only by the first library returned from `both()`
    _guard: Option<MutexGuard<'static, ()>>,
    _lib: Library,
    pub arr_push: FnArrPush,
    pub strkey: FnStrkey,
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
}

macro_rules! sym {
    ($lib:expr, $name:literal, $ty:ty) => {{
        let s: Symbol<$ty> = unsafe {
            $lib.get(concat!($name, "\0").as_bytes())
                .unwrap_or_else(|e| panic!("missing symbol {}: {}", $name, e))
        };
        *s
    }};
}

impl Lib {
    pub fn open(name: &'static str, path: &Path) -> Lib {
        Lib::open_with(name, path, None)
    }

    fn open_with(
        name: &'static str,
        path: &Path,
        guard: Option<MutexGuard<'static, ()>>,
    ) -> Lib {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("cannot dlopen {}: {}", path.display(), e));
        let l = Lib {
            name,
            _guard: guard,
            arr_push: sym!(lib, "arr_push", FnArrPush),
            strkey: sym!(lib, "strkey", FnStrkey),
            rand_seed: sym!(lib, "stbds_rand_seed", FnRandSeed),
            hash_bytes: sym!(lib, "stbds_hash_bytes", FnHashBytes),
            hash_string: sym!(lib, "stbds_hash_string", FnHashString),
            arrgrowf: sym!(lib, "stbds_arrgrowf", FnArrGrowf),
            arrfreef: sym!(lib, "stbds_arrfreef", FnArrFreef),
            hmfree_func: sym!(lib, "stbds_hmfree_func", FnHmFree),
            hmget_key: sym!(lib, "stbds_hmget_key", FnHmGetKey),
            hmget_key_ts: sym!(lib, "stbds_hmget_key_ts", FnHmGetKeyTs),
            hmput_default: sym!(lib, "stbds_hmput_default", FnHmPutDefault),
            hmput_key: sym!(lib, "stbds_hmput_key", FnHmPutKey),
            hmdel_key: sym!(lib, "stbds_hmdel_key", FnHmDelKey),
            shmode_func: sym!(lib, "stbds_shmode_func", FnShModeFunc),
            stralloc: sym!(lib, "stbds_stralloc", FnStrAlloc),
            strreset: sym!(lib, "stbds_strreset", FnStrReset),
            _lib: lib,
        };
        l
    }
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn find_c_so() -> PathBuf {
    let dir = repo_root().join("c_src/build");
    let mut cands: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("c_src/build not built ({}): {}", dir.display(), e))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    cands.sort();
    assert!(!cands.is_empty(), "no .so found in {}", dir.display());
    cands.remove(0)
}

/// `cargo test` does **not** rebuild a `crate-type = ["cdylib"]` artifact (the
/// lib target is compiled as a test harness instead), so without this guard the
/// whole suite would happily differential-test a STALE `.so` and report green
/// for a Rust source tree it never actually exercised. Refuse to run in that
/// case; `run_all.sh` always does `cargo build` before `cargo test`.
fn assert_so_is_fresh(so: &Path) {
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("cannot stat {}: {}", so.display(), e));

    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    let mut consider = |p: PathBuf| {
        if let Ok(t) = std::fs::metadata(&p).and_then(|m| m.modified()) {
            if newest.as_ref().map(|(_, n)| t > *n).unwrap_or(true) {
                newest = Some((p, t));
            }
        }
    };
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    consider(crate_dir.join("Cargo.toml"));
    let mut stack = vec![crate_dir.join("src")];
    while let Some(dir) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().map(|x| x == "rs").unwrap_or(false) {
                    consider(p);
                }
            }
        }
    }

    if let Some((newest_path, newest_time)) = newest {
        assert!(
            so_mtime >= newest_time,
            "STALE Rust .so: {} is older than {}.\n\
             `cargo test` does not rebuild a cdylib -- run `cargo build --release` \n\
             first (or just use ./run_all.sh), otherwise these differential tests \n\
             would validate a binary that does not match src/.",
            so.display(),
            newest_path.display()
        );
    }
}

fn find_rust_so() -> PathBuf {
    // The test binary lives in target/<profile>/deps/, so walk up to the
    // profile dir and pick the cdylib next to it.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("profile dir")
        .to_path_buf();
    let p = profile_dir.join("libarr_push_lib.so");
    if p.exists() {
        assert_so_is_fresh(&p);
        return p;
    }
    // fall back to release / debug
    for pr in ["release", "debug"] {
        let q = repo_root()
            .join("translation/target")
            .join(pr)
            .join("libarr_push_lib.so");
        if q.exists() {
            assert_so_is_fresh(&q);
            return q;
        }
    }
    panic!("libarr_push_lib.so not found (expected {})", p.display());
}

/// Opens both libraries and takes the process-wide serialisation lock (held
/// until the returned values drop). Always returns `(c, rust)`.
pub fn both() -> (Lib, Lib) {
    let g = serial();
    let c = Lib::open_with("C", &find_c_so(), Some(g));
    let r = Lib::open("RUST", &find_rust_so());
    (c, r)
}

// ---------------------------------------------------------------------------
// State snapshotting (pointers differ between heaps, so compare *contents*)
// ---------------------------------------------------------------------------

unsafe fn rd_usize(p: *const u8, off: usize) -> usize {
    (p.add(off) as *const usize).read_unaligned()
}
unsafe fn rd_isize(p: *const u8, off: usize) -> isize {
    (p.add(off) as *const isize).read_unaligned()
}
unsafe fn rd_ptr(p: *const u8, off: usize) -> *mut c_void {
    (p.add(off) as *const *mut c_void).read_unaligned()
}

pub unsafe fn cstr_bytes(p: *const c_char) -> Vec<u8> {
    if p.is_null() {
        return Vec::new();
    }
    let mut v = Vec::new();
    let mut q = p as *const u8;
    while *q != 0 {
        v.push(*q);
        q = q.add(1);
    }
    v
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BucketSnap {
    pub hash: [usize; BUCKET_LEN],
    pub index: [isize; BUCKET_LEN],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableSnap {
    pub temp_key_null: bool,
    pub temp_key_str: Vec<u8>,
    pub slot_count: usize,
    pub used_count: usize,
    pub used_count_threshold: usize,
    pub used_count_shrink_threshold: usize,
    pub tombstone_count: usize,
    pub tombstone_count_threshold: usize,
    pub seed: usize,
    pub slot_count_log2: usize,
    pub str_storage_null: bool,
    pub str_remaining: usize,
    pub str_block: u8,
    pub str_mode: u8,
    pub buckets: Vec<BucketSnap>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapSnap {
    pub null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    /// Element payloads. For pointer-keyed maps the first 8 bytes (the key
    /// pointer) are replaced by the *string it points at*, so the snapshot is
    /// heap-address independent.
    pub elems: Vec<(Option<Vec<u8>>, Vec<u8>)>,
    pub table: Option<TableSnap>,
}

/// How the key is stored inside an element, so the snapshot can be made
/// address-independent.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyKind {
    /// key is `keysize` raw bytes at offset 0
    Raw,
    /// key is a `char *` at offset 0
    Ptr,
}

/// Snapshot the observable state reachable from a hash-biased handle `h`
/// (what every `stbds_hm*` function returns).
///
/// `deref_temp_key` must be `false` unless the caller knows
/// `stbds_temp_key` has actually been written by a preceding string-mode
/// insert: `stbds_make_hash_index` leaves `stbds_hash_index::temp_key`
/// **uninitialised** in the C original, so dereferencing it otherwise reads
/// garbage in *both* libraries.
pub unsafe fn snap_map(
    h: *mut c_void,
    elemsize: usize,
    kind: KeyKind,
    deref_temp_key: bool,
) -> MapSnap {
    if h.is_null() {
        return MapSnap {
            null: true,
            length: 0,
            capacity: 0,
            temp: 0,
            elems: Vec::new(),
            table: None,
        };
    }
    let raw = (h as *mut u8).sub(elemsize);
    let hdr = raw.sub(HEADER_SIZE) as *const u8;
    let length = rd_usize(hdr, OFF_LENGTH);
    let capacity = rd_usize(hdr, OFF_CAPACITY);
    let temp = rd_isize(hdr, OFF_TEMP);
    let table_ptr = rd_ptr(hdr, OFF_HASH_TABLE);

    let mut elems = Vec::new();
    for i in 0..length {
        let e = raw.add(elemsize * i);
        match kind {
            KeyKind::Raw => {
                let mut v = Vec::with_capacity(elemsize);
                for k in 0..elemsize {
                    v.push(*e.add(k));
                }
                elems.push((None, v));
            }
            KeyKind::Ptr => {
                let kp = (e as *const *const c_char).read_unaligned();
                let ks = if kp.is_null() {
                    None
                } else {
                    Some(cstr_bytes(kp))
                };
                let mut v = Vec::new();
                for k in 8..elemsize {
                    v.push(*e.add(k));
                }
                elems.push((ks, v));
            }
        }
    }

    let table = if table_ptr.is_null() {
        None
    } else {
        let t = table_ptr as *const u8;
        let slot_count = rd_usize(t, HI_SLOT_COUNT);
        let storage = rd_ptr(t, HI_STORAGE) as *const u8;
        let nbuckets = slot_count >> BUCKET_SHIFT;
        let mut buckets = Vec::with_capacity(nbuckets);
        for b in 0..nbuckets {
            let bp = storage.add(b * 128);
            let mut hash = [0usize; BUCKET_LEN];
            let mut index = [0isize; BUCKET_LEN];
            for j in 0..BUCKET_LEN {
                hash[j] = rd_usize(bp, j * 8);
                index[j] = rd_isize(bp, 64 + j * 8);
            }
            buckets.push(BucketSnap { hash, index });
        }
        let tk = rd_ptr(t, HI_TEMP_KEY) as *const c_char;
        Some(TableSnap {
            temp_key_null: if deref_temp_key { tk.is_null() } else { false },
            temp_key_str: if deref_temp_key && !tk.is_null() {
                cstr_bytes(tk)
            } else {
                Vec::new()
            },
            slot_count,
            used_count: rd_usize(t, HI_USED_COUNT),
            used_count_threshold: rd_usize(t, HI_USED_COUNT_THRESHOLD),
            used_count_shrink_threshold: rd_usize(t, HI_USED_COUNT_SHRINK_THRESHOLD),
            tombstone_count: rd_usize(t, HI_TOMBSTONE_COUNT),
            tombstone_count_threshold: rd_usize(t, HI_TOMBSTONE_COUNT_THRESHOLD),
            seed: rd_usize(t, HI_SEED),
            slot_count_log2: rd_usize(t, HI_SLOT_COUNT_LOG2),
            str_storage_null: rd_ptr(t, HI_STRING + SA_STORAGE).is_null(),
            str_remaining: rd_usize(t, HI_STRING + SA_REMAINING),
            str_block: *t.add(HI_STRING + SA_BLOCK),
            str_mode: *t.add(HI_STRING + SA_MODE),
            buckets,
        })
    };

    MapSnap {
        null: false,
        length,
        capacity,
        temp,
        elems,
        table,
    }
}

/// Snapshot a plain dynamic array handle (as returned by `stbds_arrgrowf`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArrSnap {
    pub null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub hash_table_null: bool,
    pub bytes: Vec<u8>,
}

pub unsafe fn snap_arr(a: *mut c_void, elemsize: usize) -> ArrSnap {
    if a.is_null() {
        return ArrSnap {
            null: true,
            length: 0,
            capacity: 0,
            temp: 0,
            hash_table_null: true,
            bytes: Vec::new(),
        };
    }
    let hdr = (a as *const u8).sub(HEADER_SIZE);
    let length = rd_usize(hdr, OFF_LENGTH);
    let mut bytes = Vec::new();
    for k in 0..length * elemsize {
        bytes.push(*(a as *const u8).add(k));
    }
    ArrSnap {
        null: false,
        length,
        capacity: rd_usize(hdr, OFF_CAPACITY),
        temp: rd_isize(hdr, OFF_TEMP),
        hash_table_null: rd_ptr(hdr, OFF_HASH_TABLE).is_null(),
        bytes,
    }
}

/// Address-independent snapshot of a `stbds_string_arena`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArenaSnap {
    pub storage_null: bool,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
    /// number of blocks in the chain
    pub nblocks: usize,
}

pub unsafe fn snap_arena(a: *const StringArena) -> ArenaSnap {
    let mut n = 0usize;
    let mut x = (*a).storage;
    while !x.is_null() && n < 1_000_000 {
        n += 1;
        x = (x as *const *mut c_void).read_unaligned();
    }
    ArenaSnap {
        storage_null: (*a).storage.is_null(),
        remaining: (*a).remaining,
        block: (*a).block,
        mode: (*a).mode,
        nblocks: n,
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
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
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 40) as u8
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
    /// Random NUL-terminated ASCII-ish string of `n` payload bytes
    /// (bytes are 1..=255 so they never terminate early).
    pub fn cstring(&mut self, n: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..n).map(|_| 1 + (self.byte() % 255)).collect();
        v.push(0);
        v
    }
    pub fn ascii_cstring(&mut self, n: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..n).map(|_| b'a' + (self.byte() % 26)).collect();
        v.push(0);
        v
    }
}

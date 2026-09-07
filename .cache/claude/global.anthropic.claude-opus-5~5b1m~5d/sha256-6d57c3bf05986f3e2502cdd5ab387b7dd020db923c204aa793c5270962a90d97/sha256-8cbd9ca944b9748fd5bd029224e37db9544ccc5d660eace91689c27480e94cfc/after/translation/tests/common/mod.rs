//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` through `libloading` and exposes
//! the 16 exported symbols as typed function pointers.  No Rust function is
//! ever called directly — every call crosses the FFI boundary exactly as an
//! external consumer's would, so the `#[no_mangle]` wrappers are under test
//! too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// layout-mirrors of the C structs (for white-box state comparison)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Header {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

pub const HDR: usize = std::mem::size_of::<Header>();

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Arena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

impl Arena {
    pub fn zeroed() -> Arena {
        Arena {
            storage: std::ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        }
    }
}

pub const BUCKET_LEN: usize = 8;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct HashBucket {
    pub hash: [usize; BUCKET_LEN],
    pub index: [isize; BUCKET_LEN],
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
    pub string: Arena,
    pub storage: *mut HashBucket,
}

// modes
pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;
pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

pub const DEFAULT_SEED: usize = 0x3141_5926;

// ---------------------------------------------------------------------------
// loaded library
// ---------------------------------------------------------------------------

type FnHelxo = unsafe extern "C" fn(c_char);
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
type FnStrAlloc = unsafe extern "C" fn(*mut Arena, *mut c_char) -> *mut c_char;
type FnStrReset = unsafe extern "C" fn(*mut Arena);

pub struct Lib {
    pub name: &'static str,
    _lib: Library,
    pub helxo: FnHelxo,
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

unsafe fn sym<T: Copy>(lib: &Library, n: &[u8]) -> T {
    let s: Symbol<T> = lib
        .get(n)
        .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(n)));
    *s
}

impl Lib {
    unsafe fn open(name: &'static str, path: &PathBuf) -> Lib {
        let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {path:?}: {e}"));
        Lib {
            name,
            helxo: sym(&lib, b"helxo\0"),
            strkey: sym(&lib, b"strkey\0"),
            rand_seed: sym(&lib, b"stbds_rand_seed\0"),
            hash_bytes: sym(&lib, b"stbds_hash_bytes\0"),
            hash_string: sym(&lib, b"stbds_hash_string\0"),
            arrgrowf: sym(&lib, b"stbds_arrgrowf\0"),
            arrfreef: sym(&lib, b"stbds_arrfreef\0"),
            hmfree_func: sym(&lib, b"stbds_hmfree_func\0"),
            hmget_key: sym(&lib, b"stbds_hmget_key\0"),
            hmget_key_ts: sym(&lib, b"stbds_hmget_key_ts\0"),
            hmput_default: sym(&lib, b"stbds_hmput_default\0"),
            hmput_key: sym(&lib, b"stbds_hmput_key\0"),
            hmdel_key: sym(&lib, b"stbds_hmdel_key\0"),
            shmode_func: sym(&lib, b"stbds_shmode_func\0"),
            stralloc: sym(&lib, b"stbds_stralloc\0"),
            strreset: sym(&lib, b"stbds_strreset\0"),
            _lib: lib,
        }
    }
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn find_c_so() -> PathBuf {
    let build = crate_root().join("../c_src/build");
    let mut found: Option<PathBuf> = None;
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("lib")
            {
                found = Some(p);
            }
        }
    }
    found.unwrap_or_else(|| {
        panic!("no C .so under {build:?} — build it with cmake first (see task instructions)")
    })
}

pub fn find_rust_so() -> PathBuf {
    // `HELXO_RUST_SO` lets the same suite be re-run against the release
    // artifact (opt-level 3 + panic=abort), which is what actually ships.
    if let Ok(v) = std::env::var("HELXO_RUST_SO") {
        let p = PathBuf::from(v);
        assert!(p.exists(), "HELXO_RUST_SO points at a missing file: {p:?}");
        return p;
    }
    let root = crate_root();
    let cands = [
        root.join("target/debug/libhelxo_lib.so"),
        root.join("target/release/libhelxo_lib.so"),
    ];
    for c in cands.iter() {
        if c.exists() {
            return c.clone();
        }
    }
    panic!("no Rust libhelxo_lib.so found in target/{{debug,release}}");
}

pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

static PAIR: OnceLock<Pair> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());

/// Serialises access: both `.so`s carry mutable process-global state
/// (`stbds_hash_seed`, the `strkey` static buffer, stdout), so tests must not
/// interleave.
pub fn libs() -> (&'static Pair, MutexGuard<'static, ()>) {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let p = PAIR.get_or_init(|| unsafe {
        Pair {
            c: Lib::open("C", &find_c_so()),
            r: Lib::open("Rust", &find_rust_so()),
        }
    });
    (p, g)
}

/// Reset the global hash seed in BOTH libraries so their tables draw the same
/// seeds. Must be called at the start of every scenario.
pub fn reseed(p: &Pair, seed: usize) {
    unsafe {
        (p.c.rand_seed)(seed);
        (p.r.rand_seed)(seed);
    }
}

// ---------------------------------------------------------------------------
// deterministic RNG (xoshiro256**-ish; fixed seed per test for reproducibility)
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn u32(&mut self) -> u32 {
        self.next_u64() as u32
    }
    pub fn u8(&mut self) -> u8 {
        self.next_u64() as u8
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.u8()).collect()
    }
    /// Random NUL-free byte string of length `n` (any value 1..=255).
    pub fn cstr_bytes(&mut self, n: usize, high_bit: bool) -> Vec<u8> {
        let mut v = Vec::with_capacity(n + 1);
        for _ in 0..n {
            let b = if high_bit {
                let x = self.u8();
                if x == 0 {
                    1
                } else {
                    x
                }
            } else {
                b'a' + (self.u8() % 26)
            };
            v.push(b);
        }
        v.push(0);
        v
    }
}

// ---------------------------------------------------------------------------
// state snapshots
// ---------------------------------------------------------------------------

pub unsafe fn hdr_of(arr: *mut c_void) -> Header {
    *((arr as *mut u8).sub(HDR) as *mut Header)
}

/// `map` is the user-visible pointer (`arr + elemsize`).
pub unsafe fn map_arr(map: *mut c_void, elemsize: usize) -> *mut c_void {
    (map as *mut u8).sub(elemsize) as *mut c_void
}

#[derive(Debug, PartialEq, Eq)]
pub struct BucketSnap {
    pub hash: Vec<usize>,
    pub index: Vec<isize>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct IdxSnap {
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
    pub buckets: Vec<BucketSnap>,
}

pub unsafe fn idx_snap(t: *mut HashIndex) -> Option<IdxSnap> {
    if t.is_null() {
        return None;
    }
    let n = (*t).slot_count >> 3;
    let mut buckets = Vec::with_capacity(n);
    for i in 0..n {
        let b = &*(*t).storage.add(i);
        buckets.push(BucketSnap {
            hash: b.hash.to_vec(),
            index: b.index.to_vec(),
        });
    }
    Some(IdxSnap {
        slot_count: (*t).slot_count,
        used_count: (*t).used_count,
        used_count_threshold: (*t).used_count_threshold,
        used_count_shrink_threshold: (*t).used_count_shrink_threshold,
        tombstone_count: (*t).tombstone_count,
        tombstone_count_threshold: (*t).tombstone_count_threshold,
        seed: (*t).seed,
        slot_count_log2: (*t).slot_count_log2,
        arena_remaining: (*t).string.remaining,
        arena_block: (*t).string.block,
        arena_mode: (*t).string.mode,
        arena_has_storage: !(*t).string.storage.is_null(),
        buckets,
    })
}

/// How the key field of an element should be compared.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum KeyKind {
    /// key is `keysize` raw bytes at `keyoffset` — compare bytes.
    Bytes,
    /// key is a `char*` at `keyoffset` — compare the pointed-to C string
    /// (the pointers themselves legitimately differ between the two heaps).
    CStr,
}

/// Full observable state of a map, normalised so the two libraries are
/// comparable (heap addresses are never compared, only contents).
#[derive(Debug, PartialEq, Eq)]
pub struct MapSnap {
    pub is_null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    /// per element (index 1..length): non-key bytes then the key rendering
    pub elems: Vec<(Vec<u8>, Vec<u8>)>,
    pub idx: Option<IdxSnap>,
}

/// `stbds_temp_key(a)` == `*(char **) header(a)->hash_table`, i.e. the
/// `temp_key` field of the hash index.
///
/// NOTE: the C never initialises this field — `stbds_make_hash_index` leaves
/// it as whatever `realloc` returned — so it is only meaningful immediately
/// after a `stbds_hmput_key` call in string mode with
/// `string.mode in {SH_DEFAULT, SH_STRDUP, SH_ARENA}`.  It must therefore be
/// read at those points only, never as part of a general snapshot.
pub unsafe fn temp_key_str(map: *mut c_void, elemsize: usize) -> Option<Vec<u8>> {
    if map.is_null() {
        return None;
    }
    let h = hdr_of(map_arr(map, elemsize));
    let t = h.hash_table as *mut HashIndex;
    if t.is_null() {
        return None;
    }
    let tk = (*t).temp_key;
    if tk.is_null() {
        Some(b"<null>".to_vec())
    } else {
        Some(cstr_bytes(tk))
    }
}

pub unsafe fn cstr_bytes(p: *const c_char) -> Vec<u8> {
    if p.is_null() {
        return b"<null>".to_vec();
    }
    let mut v = Vec::new();
    let mut q = p as *const u8;
    while *q != 0 {
        v.push(*q);
        q = q.add(1);
    }
    v
}

pub unsafe fn map_snap(
    map: *mut c_void,
    elemsize: usize,
    keyoffset: usize,
    keysize: usize,
    kind: KeyKind,
) -> MapSnap {
    if map.is_null() {
        return MapSnap {
            is_null: true,
            length: 0,
            capacity: 0,
            temp: 0,
            elems: vec![],
            idx: None,
        };
    }
    let arr = map_arr(map, elemsize);
    let h = hdr_of(arr);
    let mut elems = Vec::new();
    let base = arr as *mut u8;
    for i in 1..h.length {
        let e = base.add(elemsize * i);
        let raw = std::slice::from_raw_parts(e, elemsize).to_vec();
        let (keybytes, other) = match kind {
            KeyKind::Bytes => {
                let k = raw[keyoffset..keyoffset + keysize].to_vec();
                let mut o = raw.clone();
                for b in o[keyoffset..keyoffset + keysize].iter_mut() {
                    *b = 0;
                }
                (k, o)
            }
            KeyKind::CStr => {
                let kp = *(e.add(keyoffset) as *mut *mut c_char);
                let k = cstr_bytes(kp);
                let mut o = raw.clone();
                // zero the pointer field: addresses differ between heaps
                for b in o[keyoffset..keyoffset + std::mem::size_of::<usize>()].iter_mut() {
                    *b = 0;
                }
                (k, o)
            }
        };
        elems.push((other, keybytes));
    }
    let t = h.hash_table as *mut HashIndex;
    MapSnap {
        is_null: false,
        length: h.length,
        capacity: h.capacity,
        temp: h.temp,
        elems,
        idx: idx_snap(t),
    }
}

// ---------------------------------------------------------------------------
// stdout capture (for `helxo`)
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(f: *mut c_void) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn lseek(fd: c_int, off: i64, whence: c_int) -> i64;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    fn unlink(path: *const c_char) -> c_int;
}

const O_RDWR: c_int = 2;
const O_CREAT: c_int = 64;
const O_TRUNC: c_int = 512;

/// Runs `f` with fd 1 redirected to a scratch file and returns everything the
/// library wrote to stdout.
pub fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    unsafe {
        let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
        let path = format!("{dir}/helxo_cap_{tag}_{}.txt\0", std::process::id());
        let fd = open(path.as_ptr() as *const c_char, O_RDWR | O_CREAT | O_TRUNC, 0o600i32);
        assert!(fd >= 0, "open scratch file failed");
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0);
        assert!(dup2(fd, 1) >= 0);

        f();

        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);

        lseek(fd, 0, 0);
        let mut out = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = read(fd, buf.as_mut_ptr() as *mut c_void, buf.len());
            if n <= 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
        close(fd);
        unlink(path.as_ptr() as *const c_char);
        out
    }
}

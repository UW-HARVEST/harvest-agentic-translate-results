//! Differential-test harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and calls every entry point through its exported symbol only.
//! Nothing in this module ever calls a Rust function directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits the harness itself needs (stdout capture).
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn fflush(f: *mut c_void) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

// ---------------------------------------------------------------------------
// ABI-identical mirrors of the C structures (for state inspection only).
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringArena {
    pub storage: *mut c_void,
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

pub const HEADER_SIZE: usize = std::mem::size_of::<ArrayHeader>();

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;
pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

// ---------------------------------------------------------------------------
// Function-pointer types (exactly the C prototypes).
// ---------------------------------------------------------------------------
type FnArrGrowF = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type FnArrFreeF = unsafe extern "C" fn(*mut c_void);
type FnRandSeed = unsafe extern "C" fn(usize);
type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
type FnHmFreeFunc = unsafe extern "C" fn(*mut c_void, usize);
type FnHmGetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
type FnHmGetKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type FnHmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FnHmPutKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type FnShModeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
type FnHmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
type FnStrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
type FnStrReset = unsafe extern "C" fn(*mut StringArena);
type FnStrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
type FnShPuts = unsafe extern "C" fn(c_int);

/// One loaded shared object with all 16 exported symbols resolved.
pub struct Api {
    pub name: &'static str,
    _lib: Library,
    pub arrgrowf: FnArrGrowF,
    pub arrfreef: FnArrFreeF,
    pub rand_seed: FnRandSeed,
    pub hash_string: FnHashString,
    pub hash_bytes: FnHashBytes,
    pub hmfree_func: FnHmFreeFunc,
    pub hmget_key_ts: FnHmGetKeyTs,
    pub hmget_key: FnHmGetKey,
    pub hmput_default: FnHmPutDefault,
    pub hmput_key: FnHmPutKey,
    pub shmode_func: FnShModeFunc,
    pub hmdel_key: FnHmDelKey,
    pub stralloc: FnStrAlloc,
    pub strreset: FnStrReset,
    pub strkey: FnStrKey,
    pub sh_puts: FnShPuts,
}

unsafe fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
    unsafe {
        let s: Symbol<T> = lib
            .get(name)
            .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)));
        *s
    }
}

impl Api {
    unsafe fn load(name: &'static str, path: &PathBuf) -> Api {
        unsafe {
            let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {path:?}: {e}"));
            Api {
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
                sh_puts: sym(&lib, b"sh_puts\0"),
                _lib: lib,
            }
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let dir = manifest_dir().parent().unwrap().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("c_src/build not built ({dir:?}): {e}"))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(found.len(), 1, "expected exactly one .so in {dir:?}: {found:?}");
    found.pop().unwrap()
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("target/release/libsh_puts_lib.so");
    assert!(p.exists(), "run `cargo build --release` first ({p:?})");
    p
}

pub struct Pair {
    pub c: Api,
    pub r: Api,
}

static PAIR: OnceLock<Pair> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());

/// Serialising guard.  Both `.so`s carry mutable global state
/// (`stbds_hash_seed`, `buffer`), so only one test may drive them at a time.
/// Acquiring the guard also re-seeds both libraries to the same value so every
/// test starts from an identical global state.
pub struct Session<'a> {
    _g: MutexGuard<'a, ()>,
    pub c: &'static Api,
    pub r: &'static Api,
}

thread_local! {
    static HELD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub fn session(seed: usize) -> Session<'static> {
    assert!(
        !HELD.with(|h| h.get()),
        "nested session(): the outer Session must be dropped first (this would \
         self-deadlock on the serialising mutex)"
    );
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    HELD.with(|h| h.set(true));
    let pair = PAIR.get_or_init(|| unsafe {
        Pair {
            c: Api::load("C", &c_so_path()),
            r: Api::load("RUST", &rust_so_path()),
        }
    });
    unsafe {
        (pair.c.rand_seed)(seed);
        (pair.r.rand_seed)(seed);
    }
    Session {
        _g: g,
        c: &pair.c,
        r: &pair.r,
    }
}

impl Drop for Session<'_> {
    fn drop(&mut self) {
        HELD.with(|h| h.set(false));
    }
}

impl Session<'_> {
    /// Re-seed both libraries mid-test.
    pub fn seed(&self, s: usize) {
        unsafe {
            (self.c.rand_seed)(s);
            (self.r.rand_seed)(s);
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) — fixed seed per row for reproducibility.
// ---------------------------------------------------------------------------
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E3779B97F4A7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next_u64() % n as u64) as usize }
    }
    pub fn range(&mut self, lo: usize, hi_incl: usize) -> usize {
        lo + self.below(hi_incl - lo + 1)
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
    /// Random NUL-free byte string of length `n` (bytes 0x01..=0xFF).
    pub fn cstr_body(&mut self, n: usize) -> Vec<u8> {
        (0..n)
            .map(|_| {
                let b = self.byte();
                if b == 0 { 1 } else { b }
            })
            .collect()
    }
    /// Random printable ASCII string of random length in `lo..=hi`.
    pub fn ascii_range(&mut self, lo: usize, hi_incl: usize) -> Vec<u8> {
        let n = self.range(lo, hi_incl);
        self.ascii(n)
    }
    /// Random printable ASCII string of length `n`.
    pub fn ascii(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| b'!' + (self.below(94) as u8)).collect()
    }
}

/// NUL-terminated owned C string buffer.
pub struct CStrBuf(pub Vec<u8>);

impl CStrBuf {
    pub fn new(body: &[u8]) -> CStrBuf {
        let mut v = body.to_vec();
        v.push(0);
        // Slack: the `STBDS_SH_NONE` code path `memcpy`s `keysize` bytes out of
        // the key regardless of where the NUL is, so the buffer must have at
        // least `keysize` readable bytes.  Zero padding keeps both libraries
        // reading identical, defined bytes.
        v.extend_from_slice(&[0u8; 24]);
        CStrBuf(v)
    }
    pub fn ptr(&self) -> *mut c_char {
        self.0.as_ptr() as *mut c_char
    }
}

// ---------------------------------------------------------------------------
// State snapshots.  Everything observable about a map, in a comparable form.
// ---------------------------------------------------------------------------

pub unsafe fn header(raw: *mut c_void) -> ArrayHeader {
    unsafe { *((raw as *mut u8).wrapping_sub(HEADER_SIZE) as *mut ArrayHeader) }
}

/// `map` is the hash-side pointer (`arr + elemsize`); returns its raw base.
pub fn raw_of(map: *mut c_void, elemsize: usize) -> *mut c_void {
    (map as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

#[derive(Debug, PartialEq, Eq)]
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
    pub arena_present: bool,
    pub storage_aligned: bool,
    pub hashes: Vec<usize>,
    pub indices: Vec<isize>,
}

pub unsafe fn table_snap(raw: *mut c_void) -> Option<TableSnap> {
    unsafe {
        let h = header(raw);
        if h.hash_table.is_null() {
            return None;
        }
        let t = h.hash_table as *mut HashIndex;
        let nb = (*t).slot_count >> 3;
        let mut hashes = Vec::with_capacity(nb * 8);
        let mut indices = Vec::with_capacity(nb * 8);
        for i in 0..nb {
            let b = (*t).storage.add(i);
            hashes.extend_from_slice(&(*b).hash);
            indices.extend_from_slice(&(*b).index);
        }
        Some(TableSnap {
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
            arena_present: !(*t).string.storage.is_null(),
            storage_aligned: ((*t).storage as usize) % 64 == 0,
            hashes,
            indices,
        })
    }
}

/// How the element's key should be compared.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyKind {
    /// Raw bytes: compare the whole element verbatim.
    Binary,
    /// `char *` at `keyoffset`: compare the pointed-to C string, and the rest of
    /// the element verbatim.
    StringAt(usize),
    /// Two `char *` fields (e.g. the key plus a deliberate alias used by the
    /// non-zero-`keyoffset` rows): both strings are compared and both raw
    /// pointer values are blanked.
    StringAtPair(usize, usize),
}

#[derive(Debug, PartialEq, Eq)]
pub struct MapSnap {
    pub null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub table: Option<TableSnap>,
    /// per element: (non-pointer bytes, Option<key string>)
    pub elems: Vec<(Vec<u8>, Option<Vec<u8>>)>,
}

pub unsafe fn read_cstr(p: *const c_char) -> Vec<u8> {
    unsafe {
        let mut v = Vec::new();
        let mut i = 0usize;
        loop {
            let b = *(p as *const u8).add(i);
            if b == 0 {
                break;
            }
            v.push(b);
            i += 1;
        }
        v
    }
}

/// Snapshot everything observable about a map.  `map` is the hash-side pointer.
pub unsafe fn map_snap(map: *mut c_void, elemsize: usize, kind: KeyKind) -> MapSnap {
    unsafe {
        if map.is_null() {
            return MapSnap {
                null: true,
                length: 0,
                capacity: 0,
                temp: 0,
                table: None,
                elems: Vec::new(),
            };
        }
        let raw = raw_of(map, elemsize);
        let h = header(raw);
        let mut elems = Vec::new();
        for i in 0..h.length {
            let base = (raw as *mut u8).add(i * elemsize);
            let bytes = std::slice::from_raw_parts(base, elemsize).to_vec();
            match kind {
                KeyKind::Binary => elems.push((bytes, None)),
                KeyKind::StringAt(off) => {
                    let kp = *(base.add(off) as *mut *mut c_char);
                    // element 0 is the zeroed "default" slot -> NULL key
                    let key = if kp.is_null() {
                        None
                    } else {
                        Some(read_cstr(kp))
                    };
                    let mut rest = bytes.clone();
                    // blank out the pointer field: its numeric value legitimately
                    // differs between the two heaps.
                    for b in &mut rest[off..off + 8] {
                        *b = 0;
                    }
                    elems.push((rest, key));
                }
                KeyKind::StringAtPair(o1, o2) => {
                    let mut key: Option<Vec<u8>> = None;
                    let mut rest = bytes.clone();
                    for off in [o1, o2] {
                        let kp = *(base.add(off) as *mut *mut c_char);
                        if !kp.is_null() {
                            let sv = read_cstr(kp);
                            key = Some(match key {
                                None => sv,
                                Some(mut prev) => {
                                    prev.push(b'|');
                                    prev.extend_from_slice(&sv);
                                    prev
                                }
                            });
                        }
                        for b in &mut rest[off..off + 8] {
                            *b = 0;
                        }
                    }
                    elems.push((rest, key));
                }
            }
        }
        MapSnap {
            null: false,
            length: h.length,
            capacity: h.capacity,
            temp: h.temp,
            table: table_snap(raw),
            elems,
        }
    }
}

/// Assert two snapshots match, with a descriptive context on failure.
pub fn assert_snap_eq(ctx: &str, c: &MapSnap, r: &MapSnap) {
    if c != r {
        assert_eq!(c.null, r.null, "{ctx}: null-ness differs");
        assert_eq!(c.length, r.length, "{ctx}: length differs");
        assert_eq!(c.capacity, r.capacity, "{ctx}: capacity differs");
        assert_eq!(c.temp, r.temp, "{ctx}: temp differs");
        assert_eq!(
            c.table.is_some(),
            r.table.is_some(),
            "{ctx}: hash_table presence differs"
        );
        if let (Some(ct), Some(rt)) = (&c.table, &r.table) {
            assert_eq!(ct.slot_count, rt.slot_count, "{ctx}: slot_count");
            assert_eq!(ct.used_count, rt.used_count, "{ctx}: used_count");
            assert_eq!(ct.tombstone_count, rt.tombstone_count, "{ctx}: tombstone_count");
            assert_eq!(ct.seed, rt.seed, "{ctx}: table seed");
            assert_eq!(ct.slot_count_log2, rt.slot_count_log2, "{ctx}: slot_count_log2");
            assert_eq!(
                ct.used_count_threshold, rt.used_count_threshold,
                "{ctx}: used_count_threshold"
            );
            assert_eq!(
                ct.used_count_shrink_threshold, rt.used_count_shrink_threshold,
                "{ctx}: used_count_shrink_threshold"
            );
            assert_eq!(
                ct.tombstone_count_threshold, rt.tombstone_count_threshold,
                "{ctx}: tombstone_count_threshold"
            );
            assert_eq!(ct.arena_mode, rt.arena_mode, "{ctx}: arena mode");
            assert_eq!(ct.arena_block, rt.arena_block, "{ctx}: arena block");
            assert_eq!(ct.arena_remaining, rt.arena_remaining, "{ctx}: arena remaining");
            assert_eq!(ct.arena_present, rt.arena_present, "{ctx}: arena storage presence");
            assert_eq!(ct.storage_aligned, rt.storage_aligned, "{ctx}: storage alignment");
            assert_eq!(ct.hashes, rt.hashes, "{ctx}: bucket hashes");
            assert_eq!(ct.indices, rt.indices, "{ctx}: bucket indices");
        }
        assert_eq!(c.elems.len(), r.elems.len(), "{ctx}: element count");
        for (i, (ce, re)) in c.elems.iter().zip(r.elems.iter()).enumerate() {
            assert_eq!(ce.1, re.1, "{ctx}: element {i} key string");
            assert_eq!(ce.0, re.0, "{ctx}: element {i} bytes");
        }
        panic!("{ctx}: snapshots differ but field-by-field comparison passed");
    }
}

// ---------------------------------------------------------------------------
// stdout capture (both `.so`s print through the process's own libc `printf`).
// ---------------------------------------------------------------------------
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into());
        let path = format!("{dir}/diffcap_{}_{:p}.txt", std::process::id(), &saved);
        {
            let file = std::fs::File::create(&path).expect("create capture file");
            use std::os::fd::AsRawFd;
            assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 -> stdout failed");
        }
        f();
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
        let out = std::fs::read(&path).expect("read capture file");
        let _ = std::fs::remove_file(&path);
        out
    }
}

pub mod driver;
pub use driver::Driver;

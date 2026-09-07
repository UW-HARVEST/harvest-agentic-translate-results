//! Shared differential-test harness.
//!
//! Loads BOTH the original C `.so` and the translated Rust `.so` through
//! `libloading` and exposes the exported symbols as typed function pointers.
//! Nothing in the crate is ever called directly, so the `#[no_mangle]` export
//! wrappers are part of what gets tested.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits we need in the test process itself
// ---------------------------------------------------------------------------
extern "C" {
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn malloc(n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

// ---------------------------------------------------------------------------
// Function-pointer table
// ---------------------------------------------------------------------------

pub type FnArrgrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrfreef = unsafe extern "C" fn(*mut c_void);
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHmgetKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmgetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type FnHmputDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmputKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmdelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnHmfreeFunc = unsafe extern "C" fn(*mut c_void, usize);
pub type FnShmodeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnStralloc = unsafe extern "C" fn(*mut c_void, *mut c_char) -> *mut c_char;
pub type FnStrreset = unsafe extern "C" fn(*mut c_void);
pub type FnStrkey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnShGeti = unsafe extern "C" fn(c_int);

pub struct Api {
    pub name: &'static str,
    _lib: Library,
    pub arrgrowf: FnArrgrowf,
    pub arrfreef: FnArrfreef,
    pub hash_bytes: FnHashBytes,
    pub hash_string: FnHashString,
    pub rand_seed: FnRandSeed,
    pub hmget_key: FnHmgetKey,
    pub hmget_key_ts: FnHmgetKeyTs,
    pub hmput_default: FnHmputDefault,
    pub hmput_key: FnHmputKey,
    pub hmdel_key: FnHmdelKey,
    pub hmfree_func: FnHmfreeFunc,
    pub shmode_func: FnShmodeFunc,
    pub stralloc: FnStralloc,
    pub strreset: FnStrreset,
    pub strkey: FnStrkey,
    pub sh_geti: FnShGeti,
}

unsafe fn sym<T: Copy>(lib: &Library, n: &str) -> T {
    let s: Symbol<T> = lib
        .get(n.as_bytes())
        .unwrap_or_else(|e| panic!("symbol `{n}` missing: {e}"));
    *s
}

impl Api {
    unsafe fn load(name: &'static str, path: &PathBuf) -> Api {
        let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {path:?}: {e}"));
        Api {
            name,
            arrgrowf: sym(&lib, "stbds_arrgrowf"),
            arrfreef: sym(&lib, "stbds_arrfreef"),
            hash_bytes: sym(&lib, "stbds_hash_bytes"),
            hash_string: sym(&lib, "stbds_hash_string"),
            rand_seed: sym(&lib, "stbds_rand_seed"),
            hmget_key: sym(&lib, "stbds_hmget_key"),
            hmget_key_ts: sym(&lib, "stbds_hmget_key_ts"),
            hmput_default: sym(&lib, "stbds_hmput_default"),
            hmput_key: sym(&lib, "stbds_hmput_key"),
            hmdel_key: sym(&lib, "stbds_hmdel_key"),
            hmfree_func: sym(&lib, "stbds_hmfree_func"),
            shmode_func: sym(&lib, "stbds_shmode_func"),
            stralloc: sym(&lib, "stbds_stralloc"),
            strreset: sym(&lib, "stbds_strreset"),
            strkey: sym(&lib, "strkey"),
            sh_geti: sym(&lib, "sh_geti"),
            _lib: lib,
        }
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let dir = workspace_root().join("c_src/build");
    let mut found = None;
    for e in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("read_dir {dir:?}: {e} — build the C library first")) {
        let p = e.unwrap().path();
        let n = p.file_name().unwrap().to_string_lossy().to_string();
        if n.starts_with("lib") && n.ends_with(".so") {
            found = Some(p);
        }
    }
    found.unwrap_or_else(|| panic!("no lib*.so in {dir:?} — build the C library first"))
}

fn rust_so_path() -> PathBuf {
    let p = if let Ok(p) = std::env::var("RUST_SO") {
        PathBuf::from(p)
    } else {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
        let mut found = None;
        for prof in ["debug", "release"] {
            let c = base.join(prof).join("libsh_geti_lib.so");
            if c.exists() {
                found = Some(c);
                break;
            }
        }
        found.unwrap_or_else(|| panic!("libsh_geti_lib.so not found under {base:?}"))
    };

    // `cargo test` does NOT rebuild a `cdylib`-only lib target, so without this
    // guard the whole suite can silently validate a stale `.so`.
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let so_t = std::fs::metadata(&p)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("stat {p:?}: {e}"));
    let src_t = std::fs::metadata(&src)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("stat {src:?}: {e}"));
    assert!(
        so_t >= src_t,
        "STALE LIBRARY: {p:?} is older than {src:?}.\n\
         `cargo test` does not rebuild a cdylib — run `cargo build [--release]` \
         first (or use ./verify.sh)."
    );
    p
}

pub struct Both {
    pub c: Api,
    pub r: Api,
}

static BOTH: OnceLock<Both> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());

/// Acquires the global test lock (the libraries keep mutable global hash-seed
/// state, and stdout capture needs exclusive access to fd 1) and returns the
/// loaded pair.
pub fn both() -> (MutexGuard<'static, ()>, &'static Both) {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let b = BOTH.get_or_init(|| unsafe {
        Both {
            c: Api::load("C", &c_so_path()),
            r: Api::load("Rust", &rust_so_path()),
        }
    });
    (g, b)
}

/// Re-synchronises the two libraries' internal `stbds_hash_seed` state.
pub fn seed_both(b: &Both, seed: usize) {
    unsafe {
        (b.c.rand_seed)(seed);
        (b.r.rand_seed)(seed);
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) — fixed seeds, reproducible
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 17) as u8
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
    /// Random NUL-free ASCII/high-byte string of length `n` plus a NUL.
    pub fn cstring(&mut self, n: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..n)
            .map(|_| {
                let b = self.byte();
                if b == 0 {
                    1
                } else {
                    b
                }
            })
            .collect();
        v.push(0);
        v
    }
}

// ---------------------------------------------------------------------------
// Layout mirrors (for reading back the libraries' internal state)
// ---------------------------------------------------------------------------

pub const HDRSIZE: usize = 32;
pub const BUCKET_LEN: usize = 8;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArrHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
pub struct HashBucket {
    pub hash: [usize; BUCKET_LEN],
    pub index: [isize; BUCKET_LEN],
}

#[repr(C)]
pub struct StringArena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
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

/// `stbds_header(arr_ptr)`
pub unsafe fn hdr(arr: *mut c_void) -> *const ArrHeader {
    (arr as *mut u8).sub(HDRSIZE) as *const ArrHeader
}

pub unsafe fn hash_to_arr(t: *mut c_void, elemsize: usize) -> *mut c_void {
    (t as *mut u8).sub(elemsize) as *mut c_void
}

/// A pointer-free, comparable snapshot of everything the library computes.
#[derive(Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub has_table: bool,
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
    pub temp_key: Option<Vec<u8>>,
    pub buckets: Vec<(Vec<usize>, Vec<isize>)>,
    /// element bytes, index 0..length; keys are resolved to strings in string
    /// modes so heap addresses never leak into the comparison.
    pub elems: Vec<Vec<u8>>,
}

pub unsafe fn cstr(p: *const c_char) -> Vec<u8> {
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

/// How the first `keysize` bytes of each element should be interpreted.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum KeyKind {
    /// raw bytes (binary maps)
    Raw,
    /// `char *` — dereference and compare the string contents
    Str,
}

/// Snapshot a "hash pointer" (what the `hm*` functions return), i.e. the
/// pointer one element past the array base.
///
/// `keysize`   bytes at offset 0 of each element that hold the key
/// `extra`     additional bytes after `keyspan` that the test itself wrote
///             (uninitialised padding is never compared)
/// `keyspan`   offset at which `extra` starts
pub unsafe fn snap(
    t: *mut c_void,
    elemsize: usize,
    keysize: usize,
    kind: KeyKind,
    keyspan: usize,
    extra: usize,
) -> Snapshot {
    if t.is_null() {
        return Snapshot {
            null: true,
            length: 0,
            capacity: 0,
            temp: 0,
            has_table: false,
            slot_count: 0,
            used_count: 0,
            used_count_threshold: 0,
            used_count_shrink_threshold: 0,
            tombstone_count: 0,
            tombstone_count_threshold: 0,
            seed: 0,
            slot_count_log2: 0,
            arena_remaining: 0,
            arena_block: 0,
            arena_mode: 0,
            arena_has_storage: false,
            temp_key: None,
            buckets: Vec::new(),
            elems: Vec::new(),
        };
    }
    let arr = hash_to_arr(t, elemsize);
    let h = &*hdr(arr);
    let mut s = Snapshot {
        null: false,
        length: h.length,
        capacity: h.capacity,
        temp: h.temp,
        has_table: !h.hash_table.is_null(),
        slot_count: 0,
        used_count: 0,
        used_count_threshold: 0,
        used_count_shrink_threshold: 0,
        tombstone_count: 0,
        tombstone_count_threshold: 0,
        seed: 0,
        slot_count_log2: 0,
        arena_remaining: 0,
        arena_block: 0,
        arena_mode: 0,
        arena_has_storage: false,
        temp_key: None,
        buckets: Vec::new(),
        elems: Vec::new(),
    };
    if !h.hash_table.is_null() {
        let ti = &*(h.hash_table as *const HashIndex);
        s.slot_count = ti.slot_count;
        s.used_count = ti.used_count;
        s.used_count_threshold = ti.used_count_threshold;
        s.used_count_shrink_threshold = ti.used_count_shrink_threshold;
        s.tombstone_count = ti.tombstone_count;
        s.tombstone_count_threshold = ti.tombstone_count_threshold;
        s.seed = ti.seed;
        s.slot_count_log2 = ti.slot_count_log2;
        s.arena_remaining = ti.string.remaining;
        s.arena_block = ti.string.block;
        s.arena_mode = ti.string.mode;
        s.arena_has_storage = !ti.string.storage.is_null();
        // NOTE: `temp_key` is left UNINITIALISED by `stbds_make_hash_index`,
        // so it is genuine garbage until a string-mode put writes it. It is
        // therefore excluded from the general snapshot and compared explicitly
        // (see `temp_key_str`) only where the C actually assigns it.
        s.temp_key = None;
        for i in 0..(ti.slot_count >> 3) {
            let b = &*ti.storage.add(i);
            s.buckets.push((b.hash.to_vec(), b.index.to_vec()));
        }
    }
    // Element 0 is the "default" slot (memset to 0); 1..length are live.
    for i in 0..h.length {
        let e = (arr as *mut u8).add(elemsize * i);
        let mut bytes = Vec::new();
        match kind {
            KeyKind::Raw => bytes.extend_from_slice(std::slice::from_raw_parts(e, keysize)),
            KeyKind::Str => {
                // element 0 is the zeroed default slot, so its key pointer is
                // NULL; `cstr` renders that as `<null>`.
                let p = *(e as *const *const c_char);
                let s = cstr(p);
                bytes.extend_from_slice(&s);
            }
        }
        bytes.push(0xAA); // separator
        if extra > 0 {
            bytes.extend_from_slice(std::slice::from_raw_parts(e.add(keyspan), extra));
        }
        s.elems.push(bytes);
    }
    s
}

/// Classification of `stbds_temp_key(arr)` == `*(char **) header(arr)->hash_table`.
///
/// `stbds_make_hash_index` never initialises `temp_key`, so the field is
/// genuine garbage until a string-mode `stbds_hmput_key` assigns it — and the
/// C deliberately does *not* assign it in the "key already present" branch of
/// the second inner probe loop.  Dereferencing it is therefore not a valid
/// comparison.  Instead the tests poke a sentinel in before each put and then
/// classify what the library left behind, which is fully deterministic.
#[derive(Debug, PartialEq, Eq)]
pub enum TempKey {
    NoTable,
    /// untouched by the operation
    Sentinel,
    /// points at element `i`'s stored key pointer
    Elem(usize),
    /// points somewhere else (a stale/dangling value — never dereferenced)
    Other,
}

pub const TEMP_KEY_SENTINEL: usize = 1;

pub unsafe fn poke_temp_key(t: *mut c_void, elemsize: usize) {
    if t.is_null() {
        return;
    }
    let h = &*hdr(hash_to_arr(t, elemsize));
    if h.hash_table.is_null() {
        return;
    }
    *(h.hash_table as *mut usize) = TEMP_KEY_SENTINEL;
}

pub unsafe fn classify_temp_key(t: *mut c_void, elemsize: usize) -> TempKey {
    if t.is_null() {
        return TempKey::NoTable;
    }
    let arr = hash_to_arr(t, elemsize);
    let h = &*hdr(arr);
    if h.hash_table.is_null() {
        return TempKey::NoTable;
    }
    let v = *(h.hash_table as *const usize);
    if v == TEMP_KEY_SENTINEL {
        return TempKey::Sentinel;
    }
    for i in 0..h.length {
        let p = *((arr as *const u8).add(elemsize * i) as *const usize);
        if p == v {
            return TempKey::Elem(i);
        }
    }
    TempKey::Other
}

// ---------------------------------------------------------------------------
// stdout capture (sh_geti prints through the process's libc stdio)
// ---------------------------------------------------------------------------

/// Captures everything the given closure writes to fd 1, **in a forked child**.
///
/// A plain in-process `dup2` of fd 1 is not safe here: libtest prints its own
/// `test <name> ... ok` lines from another thread and they land in the capture
/// file, which makes the comparison flaky.  Forking means only the library's
/// own output can reach the file.  It also means the closure runs against a
/// private copy of the library's mutable global state, so a sequence that
/// depends on the hash-seed advancing must be run inside ONE call.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let (out, code, signal) = capture_stdout_status(f);
    assert_eq!(
        (code, signal),
        (Some(0), None),
        "captured child terminated abnormally (code={code:?} signal={signal:?})"
    );
    out
}

/// As `capture_stdout`, but also reports how the child terminated.
pub fn capture_stdout_status<F: FnOnce()>(f: F) -> (Vec<u8>, Option<i32>, Option<i32>) {
    unsafe {
        fflush(std::ptr::null_mut());
        let tmp = std::env::temp_dir().join(format!(
            "shgeti_cap_{}_{}.txt",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let file = std::fs::File::create(&tmp).unwrap();
        use std::os::unix::io::AsRawFd;
        let fd = file.as_raw_fd();

        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // child: nothing here may unwind back into libtest
            if dup2(fd, 1) < 0 {
                _exit(97);
            }
            f();
            fflush(std::ptr::null_mut());
            _exit(0);
        }

        let mut status: c_int = 0;
        let w = waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed");
        drop(file);
        let out = std::fs::read(&tmp).unwrap();
        let _ = std::fs::remove_file(&tmp);

        // WIFEXITED / WEXITSTATUS / WTERMSIG
        let (code, signal) = if status & 0x7f == 0x7f {
            (None, None) // stopped; not expected here
        } else if status & 0x7f == 0 {
            (Some((status >> 8) & 0xff), None)
        } else {
            (None, Some(status & 0x7f))
        };
        (out, code, signal)
    }
}

// ---------------------------------------------------------------------------
// misc helpers
// ---------------------------------------------------------------------------

/// Heap buffer that both libraries can read (they must never see a Rust
/// stack address that outlives the call, so this keeps things simple).
pub struct CBuf(pub *mut u8, pub usize);

impl CBuf {
    pub fn new(bytes: &[u8]) -> CBuf {
        unsafe {
            let n = bytes.len().max(1);
            let p = malloc(n) as *mut u8;
            assert!(!p.is_null());
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len());
            CBuf(p, bytes.len())
        }
    }
    pub fn ptr(&self) -> *mut c_void {
        self.0 as *mut c_void
    }
    pub fn cptr(&self) -> *mut c_char {
        self.0 as *mut c_char
    }
}

impl Drop for CBuf {
    fn drop(&mut self) {
        unsafe { free(self.0 as *mut c_void) }
    }
}

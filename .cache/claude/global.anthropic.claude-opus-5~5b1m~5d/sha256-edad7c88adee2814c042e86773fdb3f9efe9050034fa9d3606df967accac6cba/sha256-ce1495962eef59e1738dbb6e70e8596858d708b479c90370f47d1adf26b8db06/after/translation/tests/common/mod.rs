//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` (built from `c_src/`) and the Rust `.so` (this
//! crate's `cdylib`) with `libloading` and calls every function through the
//! FFI boundary, exactly as an external consumer would. No Rust function is
//! ever called directly.

#![allow(dead_code)]
#![allow(non_snake_case)]

use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Mirrored C layouts (must match `c_src/src/lib.c` byte for byte)
// ---------------------------------------------------------------------------

pub const HDR_SIZE: usize = std::mem::size_of::<ArrayHeader>();
pub const BUCKET_LENGTH: usize = 8;
pub const BUCKET_SHIFT: usize = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StringBlock {
    pub next: *mut StringBlock,
    pub storage: [c_char; 8],
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
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

// The C enum / macro constants.
pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;
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
type FnShModeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
type FnHmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
type FnStrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
type FnStrReset = unsafe extern "C" fn(*mut StringArena);
type FnStrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
type FnStrDups = unsafe extern "C" fn(c_int);

/// One loaded implementation (either the C one or the Rust one).
pub struct Lib {
    pub name: &'static str,
    _lib: libloading::Library,
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
    pub shmode_func: FnShModeFunc,
    pub hmdel_key: FnHmDelKey,
    pub stralloc: FnStrAlloc,
    pub strreset: FnStrReset,
    pub strkey: FnStrKey,
    pub str_dups: FnStrDups,
}

macro_rules! sym {
    ($lib:expr, $t:ty, $n:literal) => {{
        let s: libloading::Symbol<$t> = $lib
            .get(concat!($n, "\0").as_bytes())
            .unwrap_or_else(|e| panic!("missing symbol {}: {}", $n, e));
        *s
    }};
}

impl Lib {
    unsafe fn open(name: &'static str, path: &Path) -> Lib {
        let lib = libloading::Library::new(path)
            .unwrap_or_else(|e| panic!("cannot load {}: {}", path.display(), e));
        let l = Lib {
            name,
            arrgrowf: sym!(lib, FnArrGrowf, "stbds_arrgrowf"),
            arrfreef: sym!(lib, FnArrFreef, "stbds_arrfreef"),
            rand_seed: sym!(lib, FnRandSeed, "stbds_rand_seed"),
            hash_string: sym!(lib, FnHashString, "stbds_hash_string"),
            hash_bytes: sym!(lib, FnHashBytes, "stbds_hash_bytes"),
            hmfree_func: sym!(lib, FnHmFree, "stbds_hmfree_func"),
            hmget_key_ts: sym!(lib, FnHmGetKeyTs, "stbds_hmget_key_ts"),
            hmget_key: sym!(lib, FnHmGetKey, "stbds_hmget_key"),
            hmput_default: sym!(lib, FnHmPutDefault, "stbds_hmput_default"),
            hmput_key: sym!(lib, FnHmPutKey, "stbds_hmput_key"),
            shmode_func: sym!(lib, FnShModeFunc, "stbds_shmode_func"),
            hmdel_key: sym!(lib, FnHmDelKey, "stbds_hmdel_key"),
            stralloc: sym!(lib, FnStrAlloc, "stbds_stralloc"),
            strreset: sym!(lib, FnStrReset, "stbds_strreset"),
            strkey: sym!(lib, FnStrKey, "strkey"),
            str_dups: sym!(lib, FnStrDups, "str_dups"),
            _lib: lib,
        };
        l
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let m = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    m.parent().unwrap().to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                cands.push(p);
            }
        }
    }
    cands.sort();
    assert!(
        !cands.is_empty(),
        "no .so found in {} - build the C library first:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display()
    );
    cands.remove(0)
}

fn find_rust_so() -> PathBuf {
    // `RUST_SO` lets the same suite be pointed at a different build of the
    // cdylib (e.g. the release artifact) - see `run_all_configs.sh`.
    let p = if let Ok(v) = std::env::var("RUST_SO") {
        let p = PathBuf::from(v);
        assert!(p.exists(), "RUST_SO={} does not exist", p.display());
        p
    } else {
        // current_exe == <target>/<profile>/deps/<testbin>
        let exe = std::env::current_exe().expect("current_exe");
        let profile_dir = exe.parent().unwrap().parent().unwrap();
        let p = profile_dir.join("libstr_dups_lib.so");
        assert!(
            p.exists(),
            "{} does not exist - run `cargo build` first",
            p.display()
        );
        p
    };
    assert_fresh(&p);
    p
}

/// Refuse to run against a STALE cdylib.
///
/// This crate's only `crate-type` is `cdylib`, so the integration tests cannot
/// link the library - they `dlopen` it. Cargo therefore does **not** consider
/// the cdylib artifact a dependency of the test targets, and `cargo test` alone
/// will happily run the tests against whatever `libstr_dups_lib.so` a previous
/// `cargo build` left on disk. Verified empirically: editing `src/lib.rs` and
/// running `cargo test` leaves the `.so`'s md5 unchanged, so every differential
/// test passes vacuously.
///
/// That is exactly the failure mode that makes a whole verification suite
/// worthless, so it is turned into a loud, actionable error.
fn assert_fresh(so: &Path) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let so_mtime = std::fs::metadata(so).and_then(|m| m.modified()).ok();
    let mut newest_src: Option<(std::time::SystemTime, PathBuf)> = None;
    for rel in ["src/lib.rs", "Cargo.toml"] {
        let f = manifest.join(rel);
        if let Ok(t) = std::fs::metadata(&f).and_then(|m| m.modified()) {
            if newest_src.as_ref().map(|(x, _)| t > *x).unwrap_or(true) {
                newest_src = Some((t, f));
            }
        }
    }
    if let (Some(so_t), Some((src_t, src_f))) = (so_mtime, newest_src) {
        assert!(
            so_t >= src_t,
            "\n\nSTALE cdylib: {} is OLDER than {}.\n\
             `cargo test` does NOT rebuild a cdylib-only lib target, so the tests\n\
             would have been comparing the C library against an out-of-date Rust\n\
             library and passing vacuously.\n\
             Run `cargo build` (or `./run_all_configs.sh`) first.\n",
            so.display(),
            src_f.display()
        );
    }
}

/// The two loaded libraries. Loaded once per test binary.
pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
}

pub fn pair() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| unsafe {
        Pair {
            c: Lib::open("C", &find_c_so()),
            rs: Lib::open("Rust", &find_rust_so()),
        }
    })
}

/// Both libraries keep a process-global mutable `stbds_hash_seed`, and
/// `strkey` writes into a process-global `static char buffer[256]`. Cargo runs
/// tests in parallel threads inside one process, so every test must hold this
/// lock while it touches either library.
pub fn lock() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::{Mutex, OnceLock};
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) so every test is reproducible
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % (n as u64)) as usize
        }
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
    /// A NUL-terminated ASCII-ish C string of `n` payload bytes (never 0).
    pub fn cstring(&mut self, n: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..n).map(|_| 1 + (self.next_u64() % 255) as u8).collect();
        v.push(0);
        v
    }
    /// A NUL-terminated printable-ASCII C string of `n` payload bytes.
    pub fn ascii(&mut self, n: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..n).map(|_| 0x21 + (self.next_u64() % 94) as u8).collect();
        v.push(0);
        v
    }
}

// ---------------------------------------------------------------------------
// State snapshotting - turns a live map/array into a canonical byte log so
// the two implementations can be compared without comparing raw addresses.
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct Log {
    pub bytes: Vec<u8>,
    pub trace: Vec<String>,
    /// Building the human-readable `trace` dominates the runtime of the
    /// heavier property tests, so it is only populated on the *second*,
    /// diagnostic pass that `diff()` performs after a byte mismatch.
    pub tracing: bool,
    /// When >= 0, every record is written through to this fd immediately.
    ///
    /// `diff_child` needs this: a child that hits a live `STBDS_ASSERT` and
    /// `abort()`s never returns, so a log buffered in memory would be lost
    /// entirely. Flushing as we go means the surviving prefix records *how far*
    /// each implementation got before dying, which is what makes
    /// "both aborted, but at different points" detectable instead of looking
    /// like agreement.
    pub flush_fd: c_int,
    flushed: usize,
}

impl Log {
    pub fn new() -> Log {
        Log {
            bytes: Vec::new(),
            trace: Vec::new(),
            tracing: false,
            flush_fd: -1,
            flushed: 0,
        }
    }
    pub fn traced() -> Log {
        let mut l = Log::new();
        l.tracing = true;
        l
    }
    pub fn to_fd(fd: c_int) -> Log {
        let mut l = Log::new();
        l.flush_fd = fd;
        l
    }
    #[inline]
    pub fn flush(&mut self) {
        if self.flush_fd < 0 || self.flushed >= self.bytes.len() {
            return;
        }
        unsafe {
            let mut off = self.flushed;
            while off < self.bytes.len() {
                let n = libc_write(
                    self.flush_fd,
                    self.bytes.as_ptr().add(off) as *const c_void,
                    self.bytes.len() - off,
                );
                if n <= 0 {
                    break;
                }
                off += n as usize;
            }
            self.flushed = off;
        }
    }
    #[inline]
    fn t(&mut self, f: impl FnOnce() -> String) {
        if self.tracing {
            self.trace.push(f());
        }
        self.flush();
    }
    pub fn tag(&mut self, s: &str) {
        self.bytes.extend_from_slice(s.as_bytes());
        self.bytes.push(0);
        let s2 = if self.tracing { s.to_string() } else { String::new() };
        self.t(|| s2);
    }
    pub fn usz(&mut self, label: &str, v: usize) {
        self.bytes.extend_from_slice(&(v as u64).to_le_bytes());
        self.t(|| format!("{}={:#x}", label, v));
    }
    pub fn isz(&mut self, label: &str, v: isize) {
        self.bytes.extend_from_slice(&(v as i64).to_le_bytes());
        self.t(|| format!("{}={}", label, v));
    }
    pub fn i32v(&mut self, label: &str, v: i32) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
        self.t(|| format!("{}={}", label, v));
    }
    pub fn u8v(&mut self, label: &str, v: u8) {
        self.bytes.push(v);
        self.t(|| format!("{}={}", label, v));
    }
    pub fn flag(&mut self, label: &str, v: bool) {
        self.bytes.push(v as u8);
        self.t(|| format!("{}={}", label, v));
    }
    pub fn blob(&mut self, label: &str, b: &[u8]) {
        self.bytes.extend_from_slice(&(b.len() as u64).to_le_bytes());
        self.bytes.extend_from_slice(b);
        if self.tracing {
            let s = format!("{}={:02x?}", label, b);
            self.trace.push(s);
        }
        self.flush();
    }
}

/// First index at which two logs differ, with a human-readable context.
pub fn assert_logs_eq(what: &str, a: &Log, b: &Log) {
    if a.bytes == b.bytes {
        return;
    }
    // Find the first differing trace entry for a useful message.
    let mut first = usize::MAX;
    for i in 0..a.trace.len().max(b.trace.len()) {
        let x = a.trace.get(i);
        let y = b.trace.get(i);
        if x != y {
            first = i;
            break;
        }
    }
    let lo = first.saturating_sub(12);
    let hi = (first + 8).min(a.trace.len().max(b.trace.len()));
    let mut msg = format!(
        "DIVERGENCE in {}\n  first differing record index: {}\n",
        what, first
    );
    for i in lo..hi {
        let x = a.trace.get(i).map(|s| s.as_str()).unwrap_or("<none>");
        let y = b.trace.get(i).map(|s| s.as_str()).unwrap_or("<none>");
        let mark = if x != y { ">>" } else { "  " };
        msg.push_str(&format!("{} [{}] C={}\n{}      RS={}\n", mark, i, x, mark, y));
    }
    panic!("{}", msg);
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

pub unsafe fn header(raw: *mut c_void) -> *mut ArrayHeader {
    (raw as *mut u8).wrapping_sub(HDR_SIZE) as *mut ArrayHeader
}

/// Snapshot an *array* (raw pointer, i.e. the value returned by
/// `stbds_arrgrowf`).
pub unsafe fn snap_array(log: &mut Log, raw: *mut c_void, dump_bytes: usize) {
    log.tag("array");
    log.flag("null", raw.is_null());
    if raw.is_null() {
        return;
    }
    let h = header(raw);
    log.usz("length", (*h).length);
    log.usz("capacity", (*h).capacity);
    log.isz("temp", (*h).temp);
    log.flag("has_table", !(*h).hash_table.is_null());
    if dump_bytes > 0 {
        let s = std::slice::from_raw_parts(raw as *const u8, dump_bytes);
        log.blob("data", s);
    }
}

/// How element payloads should be interpreted when snapshotting.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum KeyKind {
    /// element bytes are raw (binary keys) - dump all `elemsize` bytes
    Binary,
    /// element starts with a `char *` key at `keyoffset` - dump the pointed-to
    /// string plus the remaining bytes of the element
    StringAt(usize),
}

/// Snapshot a *hash map* (the "hash pointer", i.e. `array + elemsize`, which is
/// what `stbds_hmput_key` & friends return).
///
/// `stbds_make_hash_index` never initialises `hash_index::temp_key`, so it holds
/// uninitialised `realloc` garbage until a `string.mode ∈ {DEFAULT,STRDUP,ARENA}`
/// insert (or a first-loop duplicate hit with `mode >= HM_STRING`) writes it.
/// Use `snap_map_tk` only once such a write has definitely happened.
pub unsafe fn snap_map(log: &mut Log, t: *mut c_void, elemsize: usize, kind: KeyKind) {
    snap_map_opt(log, t, elemsize, kind, false)
}

/// Like `snap_map`, but also dereferences and compares `hash_index::temp_key`.
///
/// ONLY safe when `temp_key` is provably live - use `TkValid` / `snap_map_tkv`
/// whenever duplicate puts, table growth or deletes are in play.
pub unsafe fn snap_map_tk(log: &mut Log, t: *mut c_void, elemsize: usize, kind: KeyKind) {
    snap_map_opt(log, t, elemsize, kind, true)
}

/// `snap_map`, dereferencing `temp_key` only when `tk.0` says it is live.
pub unsafe fn snap_map_tkv(
    log: &mut Log,
    t: *mut c_void,
    elemsize: usize,
    kind: KeyKind,
    tk: &TkValid,
) {
    // Record the decision itself so a divergence in liveness tracking is caught
    // rather than silently changing what gets compared.
    log.flag("tk_live", tk.0);
    snap_map_opt(log, t, elemsize, kind, tk.0)
}

/// `(length, slot_count)` of a map - `slot_count == 0` means "no hash table".
pub unsafe fn map_shape(t: *mut c_void, elemsize: usize) -> (usize, usize) {
    if t.is_null() {
        return (0, 0);
    }
    let raw = (t as *mut u8).wrapping_sub(elemsize) as *mut c_void;
    let h = header(raw);
    let table = (*h).hash_table as *mut HashIndex;
    let sc = if table.is_null() {
        0
    } else {
        (*table).slot_count
    };
    ((*h).length, sc)
}

/// Tracks whether `hash_index::temp_key` currently holds a *live, meaningful*
/// pointer, i.e. whether it may be dereferenced and compared.
///
/// `stbds_make_hash_index` (c_src/src/lib.c:385) copies `string` and `seed`
/// from the old table but leaves the brand-new table's `temp_key` as
/// uninitialised `realloc` garbage. Combined with the fact that the wrap-around
/// duplicate-hit branch (c_src/src/lib.c:746-759) deliberately does NOT refresh
/// `temp_key` - unlike the first-loop branch at line 732-733 - a duplicate put
/// that happens to trigger a table growth can leave `temp_key` pointing at
/// allocator garbage. Reading that is not library behaviour, so it must not be
/// compared.
///
/// Rules (all derived from the C):
///   * a put that inserted a NEW entry always writes `temp_key`
///     (c_src/src/lib.c:786-788)  -> live;
///   * a duplicate put with NO table growth either refreshes `temp_key`
///     (first-loop hit) or leaves the previous value untouched -> live iff it
///     was live before;
///   * a put that grew the table but did not insert -> conservatively dead;
///   * any delete may free the key (`SH_STRDUP`, line 837) and/or rebuild the
///     table (lines 854-862) -> conservatively dead.
#[derive(Copy, Clone, Debug)]
pub struct TkValid(pub bool);

impl Default for TkValid {
    fn default() -> Self {
        TkValid(false)
    }
}

impl TkValid {
    pub fn new() -> TkValid {
        TkValid(false)
    }
    pub fn after_put(&mut self, before: (usize, usize), after: (usize, usize)) {
        let inserted = after.0 != before.0;
        let grew = after.1 != before.1;
        self.0 = inserted || (self.0 && !grew);
    }
    pub fn invalidate(&mut self) {
        self.0 = false;
    }
}

pub unsafe fn snap_map_opt(
    log: &mut Log,
    t: *mut c_void,
    elemsize: usize,
    kind: KeyKind,
    deref_temp_key: bool,
) {
    log.tag("map");
    log.flag("null", t.is_null());
    if t.is_null() {
        return;
    }
    let raw = (t as *mut u8).wrapping_sub(elemsize) as *mut c_void;
    let h = header(raw);
    log.usz("length", (*h).length);
    log.usz("capacity", (*h).capacity);
    log.isz("temp", (*h).temp);
    let table = (*h).hash_table as *mut HashIndex;
    log.flag("has_table", !table.is_null());
    if !table.is_null() {
        log.usz("slot_count", (*table).slot_count);
        log.usz("used_count", (*table).used_count);
        log.usz("used_thr", (*table).used_count_threshold);
        log.usz("shrink_thr", (*table).used_count_shrink_threshold);
        log.usz("tomb", (*table).tombstone_count);
        log.usz("tomb_thr", (*table).tombstone_count_threshold);
        log.usz("seed", (*table).seed);
        log.usz("log2", (*table).slot_count_log2);
        log.flag("arena_storage", !(*table).string.storage.is_null());
        log.usz("arena_remaining", (*table).string.remaining);
        log.u8v("arena_block", (*table).string.block);
        log.u8v("arena_mode", (*table).string.mode);
        if deref_temp_key {
            log.blob("temp_key", &cstr_bytes((*table).temp_key));
        }
        let nbuckets = (*table).slot_count >> BUCKET_SHIFT;
        for i in 0..nbuckets {
            let b = (*table).storage.add(i);
            for j in 0..BUCKET_LENGTH {
                log.usz("bh", (*b).hash[j]);
            }
            for j in 0..BUCKET_LENGTH {
                log.isz("bi", (*b).index[j]);
            }
        }
    }
    // Elements. `raw` is the array base; element 0 is the "default" slot and
    // elements 1.. are the live entries (the C code's `hmlen` is length-1).
    let n = (*h).length;
    for i in 0..n {
        let e = (raw as *mut u8).wrapping_add(elemsize * i);
        match kind {
            KeyKind::Binary => {
                let s = std::slice::from_raw_parts(e as *const u8, elemsize);
                log.blob("elem", s);
            }
            KeyKind::StringAt(off) => {
                // `read_unaligned`: the C library happily stores a `char *` at
                // an arbitrary element offset when `elemsize` is not a multiple
                // of 8, so the snapshot must not assume alignment.
                let kp = (e.wrapping_add(off) as *const *const c_char).read_unaligned();
                log.blob("elem_key", &cstr_bytes(kp));
                // remaining bytes of the element, excluding the pointer field
                let mut rest = Vec::new();
                for k in 0..elemsize {
                    if k >= off && k < off + 8 {
                        continue;
                    }
                    rest.push(*e.wrapping_add(k));
                }
                log.blob("elem_rest", &rest);
            }
        }
    }
}

pub unsafe fn snap_arena(log: &mut Log, a: *const StringArena) {
    log.tag("arena");
    log.flag("storage", !(*a).storage.is_null());
    log.usz("remaining", (*a).remaining);
    log.u8v("block", (*a).block);
    log.u8v("mode", (*a).mode);
}

/// Record a `stbds_stralloc` result in a fully address-independent way:
///
///   * the string content that was copied,
///   * *where* the returned pointer sits, expressed as
///     `(chain index of the owning block, byte offset inside that block)`.
///
/// Raw addresses cannot be compared (two different `malloc`s), and a raw
/// `p - a->storage` difference is meaningless for the *oversized* path, which
/// splices a brand-new block in as `head->next` and returns a pointer into
/// **that** block, not into the head.
pub unsafe fn snap_stralloc_result(log: &mut Log, a: *const StringArena, p: *const c_char) {
    log.tag("stralloc");
    log.flag("p_null", p.is_null());
    log.blob("content", &cstr_bytes(p));

    let head = (*a).storage;
    if head.is_null() || p.is_null() {
        log.isz("blk_idx", -1);
        log.isz("blk_off", -1);
        return;
    }
    // The non-oversized path carves from the head block:
    //   p == head->storage + remaining_after
    let head_storage = std::ptr::addr_of!((*head).storage) as *const u8;
    if p as *const u8 == head_storage.wrapping_add((*a).remaining) {
        log.isz("blk_idx", 0);
        log.isz("blk_off", (8 + (*a).remaining) as isize);
        return;
    }
    // Otherwise `p` must be the very start of one of the chained blocks
    // (the oversized path returns `sb->storage`).
    let mut b = head;
    let mut idx: isize = 0;
    while !b.is_null() && idx < 1_000_000 {
        let bs = std::ptr::addr_of!((*b).storage) as *const u8;
        if p as *const u8 == bs {
            log.isz("blk_idx", idx);
            log.isz("blk_off", 8);
            return;
        }
        b = (*b).next;
        idx += 1;
    }
    log.isz("blk_idx", -2);
    log.isz("blk_off", -2);
}

// ---------------------------------------------------------------------------
// A small "consumer" that mirrors what the stb_ds macros do, so we can drive
// the low-level entry points exactly like real client code.
// ---------------------------------------------------------------------------

/// Mirrors `stbds_shput(t,k,v)` / `stbds_shputs`: put a string key then write
/// the caller's value at `value_off`, and (for the `shputs` flavour) copy the
/// canonical key pointer back out of `hash_table->temp_key`.
pub unsafe fn shput(
    lib: &Lib,
    t: *mut c_void,
    elemsize: usize,
    key: *mut c_char,
    mode: c_int,
    value_off: usize,
    value: u64,
    write_back_key: bool,
) -> *mut c_void {
    let t = (lib.hmput_key)(
        t,
        elemsize,
        key as *mut c_void,
        std::mem::size_of::<*mut c_char>(),
        mode,
    );
    let raw = (t as *mut u8).wrapping_sub(elemsize) as *mut c_void;
    let idx = (*header(raw)).temp;
    let e = (t as *mut u8).wrapping_offset(idx * elemsize as isize);
    // Fill *every* byte after the 8-byte key pointer so the element carries no
    // uninitialised `realloc` padding (which would differ between the two
    // libraries for reasons that have nothing to do with the translation).
    let mut b = value;
    for k in value_off.min(elemsize)..elemsize {
        *e.wrapping_add(k) = (b & 0xff) as u8;
        b = b.rotate_left(8);
    }
    if write_back_key {
        let table = (*header(raw)).hash_table as *mut HashIndex;
        if !table.is_null() {
            (e as *mut *mut c_char).write_unaligned((*table).temp_key);
        }
    }
    t
}

/// Mirrors `stbds_hmput(t,k,v)`: put a binary key then write the value bytes
/// into the tail of the element so that *every* byte of the element is
/// deterministic (uninitialised realloc padding would otherwise differ).
pub unsafe fn hmput(
    lib: &Lib,
    t: *mut c_void,
    elemsize: usize,
    key: &[u8],
    mode: c_int,
    value: u64,
) -> *mut c_void {
    let mut k = key.to_vec();
    let t = (lib.hmput_key)(
        t,
        elemsize,
        k.as_mut_ptr() as *mut c_void,
        key.len(),
        mode,
    );
    let raw = (t as *mut u8).wrapping_sub(elemsize) as *mut c_void;
    let idx = (*header(raw)).temp;
    let e = (t as *mut u8).wrapping_offset(idx * elemsize as isize);
    // deterministic tail: key bytes are written by the library, we fill the
    // rest of the element from `value`.
    let mut b = value;
    for k2 in key.len()..elemsize {
        *e.wrapping_add(k2) = (b & 0xff) as u8;
        b = b.rotate_left(8);
    }
    t
}

/// Mirrors `stbds_hmgeti(t,k)` for binary keys: returns `(new_t, index)`.
pub unsafe fn hmgeti(
    lib: &Lib,
    t: *mut c_void,
    elemsize: usize,
    key: &[u8],
    mode: c_int,
) -> (*mut c_void, isize) {
    let mut k = key.to_vec();
    let t = (lib.hmget_key)(
        t,
        elemsize,
        k.as_mut_ptr() as *mut c_void,
        key.len(),
        mode,
    );
    let raw = (t as *mut u8).wrapping_sub(elemsize) as *mut c_void;
    (t, (*header(raw)).temp)
}

/// Mirrors `stbds_shgeti(t,k)`.
pub unsafe fn shgeti(
    lib: &Lib,
    t: *mut c_void,
    elemsize: usize,
    key: *mut c_char,
    mode: c_int,
) -> (*mut c_void, isize) {
    let t = (lib.hmget_key)(
        t,
        elemsize,
        key as *mut c_void,
        std::mem::size_of::<*mut c_char>(),
        mode,
    );
    let raw = (t as *mut u8).wrapping_sub(elemsize) as *mut c_void;
    (t, (*header(raw)).temp)
}

/// Mirrors `stbds_hmdel(t,k)`: returns `(new_t, deleted_flag)`.
pub unsafe fn hmdel(
    lib: &Lib,
    t: *mut c_void,
    elemsize: usize,
    key_bytes: &[u8],
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> (*mut c_void, isize) {
    let mut k = key_bytes.to_vec();
    let t = (lib.hmdel_key)(
        t,
        elemsize,
        k.as_mut_ptr() as *mut c_void,
        keysize,
        keyoffset,
        mode,
    );
    if t.is_null() {
        return (t, 0);
    }
    let raw = (t as *mut u8).wrapping_sub(elemsize) as *mut c_void;
    (t, (*header(raw)).temp)
}

/// Mirrors `stbds_shdel(t,k)`.
pub unsafe fn shdel(
    lib: &Lib,
    t: *mut c_void,
    elemsize: usize,
    key: *mut c_char,
    keyoffset: usize,
    mode: c_int,
) -> (*mut c_void, isize) {
    let t = (lib.hmdel_key)(
        t,
        elemsize,
        key as *mut c_void,
        std::mem::size_of::<*mut c_char>(),
        keyoffset,
        mode,
    );
    if t.is_null() {
        return (t, 0);
    }
    let raw = (t as *mut u8).wrapping_sub(elemsize) as *mut c_void;
    (t, (*header(raw)).temp)
}

/// Mirrors `stbds_hmfree(p)`.
pub unsafe fn hmfree(lib: &Lib, t: *mut c_void, elemsize: usize) {
    if !t.is_null() {
        (lib.hmfree_func)((t as *mut u8).wrapping_sub(elemsize) as *mut c_void, elemsize);
    }
}

/// Run the same closure against both libraries and compare the produced logs.
pub fn diff<F>(what: &str, mut f: F)
where
    F: FnMut(&Lib, &mut Log),
{
    let p = pair();
    let _g = lock();
    let mut lc = Log::new();
    let mut lr = Log::new();
    f(&p.c, &mut lc);
    f(&p.rs, &mut lr);
    if lc.bytes == lr.bytes {
        return;
    }
    // Diverged: replay both sides with the human-readable trace enabled so the
    // failure message can point at the first differing record.
    let mut lc2 = Log::traced();
    let mut lr2 = Log::traced();
    f(&p.c, &mut lc2);
    f(&p.rs, &mut lr2);
    assert_logs_eq(what, &lc2, &lr2);
    // If the replay somehow matched, still fail - the first run diverged.
    panic!(
        "DIVERGENCE in {} on the first pass ({} vs {} bytes) but not on replay - \
         the scenario is not deterministic",
        what,
        lc.bytes.len(),
        lr.bytes.len()
    );
}

// ---------------------------------------------------------------------------
// stdout capture (for `str_dups`, which `printf`s)
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(f: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

// `write` is already declared for the assert path in src/lib.rs; here we need
// our own, so alias it.
extern "C" {
    #[link_name = "write"]
    fn libc_write(fd: c_int, buf: *const c_void, n: usize) -> isize;
}

/// How a forked child terminated.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Outcome {
    Exited(i32),
    Signaled(i32),
}

impl Outcome {
    fn from_status(st: c_int) -> Outcome {
        // WIFEXITED / WEXITSTATUS / WIFSIGNALED / WTERMSIG
        if st & 0x7f == 0x7f {
            Outcome::Exited(-1) // stopped; not expected here
        } else if st & 0x7f == 0 {
            Outcome::Exited((st >> 8) & 0xff)
        } else {
            Outcome::Signaled(st & 0x7f)
        }
    }
}

/// Run `f(lib)` in a forked child, capturing the byte log the child produced and
/// how the child terminated.
///
/// This is the only way to differentially test the C library's **`assert`
/// aborts**: `c_src/CMakeLists.txt` compiles without `-DNDEBUG`, so a failing
/// `STBDS_ASSERT` calls `__assert_fail` and kills the whole process (`nm -D`
/// shows `U __assert_fail`). Running each implementation in its own child lets
/// us assert that BOTH die with the same signal at the same point, instead of
/// the first abort taking the test runner down with it.
fn run_in_child<F>(lib: &Lib, f: &mut F) -> (Outcome, Vec<u8>)
where
    F: FnMut(&Lib, &mut Log),
{
    use std::io::Read;
    use std::os::unix::io::AsRawFd;

    let mut path = std::env::temp_dir();
    path.push(format!(
        "strdups_child_{}_{}_{}",
        lib.name,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("temp file");
    let fd = file.as_raw_fd();

    unsafe {
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            // --- child --- log is written through to `fd` record by record, so
            // an abort still leaves the surviving prefix behind.
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut log = Log::to_fd(fd);
                f(lib, &mut log);
                log.flush();
            }));
            fflush(std::ptr::null_mut());
            match r {
                Ok(()) => _exit(0),
                Err(_) => _exit(101),
            }
        }
        // --- parent ---
        let mut status: c_int = 0;
        let w = waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed");
        let outcome = Outcome::from_status(status);
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).ok();
        let _ = std::fs::remove_file(&path);
        (outcome, bytes)
    }
}

/// Like `diff`, but each implementation runs in its own forked child so that a
/// deliberate `assert` abort can be observed and compared instead of killing
/// the test runner.
pub fn diff_child<F>(what: &str, mut f: F)
where
    F: FnMut(&Lib, &mut Log),
{
    let p = pair();
    let _g = lock();
    let (oc, bc) = run_in_child(&p.c, &mut f);
    let (or, br) = run_in_child(&p.rs, &mut f);
    assert_eq!(
        oc, or,
        "{}: termination mismatch - C exited {:?}, Rust exited {:?}",
        what, oc, or
    );
    assert_ne!(
        oc,
        Outcome::Exited(101),
        "{}: the scenario itself panicked in the child",
        what
    );
    if bc != br {
        let n = bc.iter().zip(br.iter()).take_while(|(a, b)| a == b).count();
        panic!(
            "{}: child log mismatch after {} identical bytes (C {} bytes, Rust {} bytes)\n\
             C : {:02x?}\nRS: {:02x?}",
            what,
            n,
            bc.len(),
            br.len(),
            &bc[n.saturating_sub(32)..(n + 32).min(bc.len())],
            &br[n.saturating_sub(32)..(n + 32).min(br.len())],
        );
    }
}

/// Redirect fd 1 to a fresh temp file, run `f`, flush libc's stdout, restore
/// fd 1 and return everything that was written.
pub fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::io::AsRawFd;

    let mut path = std::env::temp_dir();
    path.push(format!(
        "strdups_cap_{}_{}_{}",
        tag,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("temp file");

    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");
        f();
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
    }

    let mut out = Vec::new();
    file.seek(SeekFrom::Start(0)).unwrap();
    file.read_to_end(&mut out).unwrap();
    let _ = std::fs::remove_file(&path);
    out
}

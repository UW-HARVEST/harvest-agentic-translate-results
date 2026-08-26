//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! called only through their exported C symbols — the Rust crate is never
//! linked directly, so the `#[no_mangle]` wrappers are part of what is tested.
//!
//!   * C   : `c_src/build/libtranslated_rust.so`
//!   * Rust: `target/<profile>/libarr_push_lib.so`

#![allow(dead_code)]

pub mod map;

use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// FFI signatures (exactly the 16 exported symbols)
// ---------------------------------------------------------------------------

pub type FnArrGrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrFreef = unsafe extern "C" fn(*mut c_void);
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHashString = unsafe extern "C" fn(*mut i8, usize) -> usize;
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHmfreeFunc = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmgetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, i32) -> *mut c_void;
pub type FnHmgetKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, i32) -> *mut c_void;
pub type FnHmputDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmputKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, i32) -> *mut c_void;
pub type FnShmodeFunc = unsafe extern "C" fn(usize, i32) -> *mut c_void;
pub type FnHmdelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, i32) -> *mut c_void;
pub type FnStralloc = unsafe extern "C" fn(*mut c_void, *mut i8) -> *mut i8;
pub type FnStrreset = unsafe extern "C" fn(*mut c_void);
pub type FnStrkey = unsafe extern "C" fn(i32) -> *mut i8;
pub type FnArrPush = unsafe extern "C" fn(i32);

/// All 16 exported symbols of one implementation.
pub struct Lib {
    pub name: &'static str,
    pub arrgrowf: FnArrGrowf,
    pub arrfreef: FnArrFreef,
    pub rand_seed: FnRandSeed,
    pub hash_string: FnHashString,
    pub hash_bytes: FnHashBytes,
    pub hmfree_func: FnHmfreeFunc,
    pub hmget_key_ts: FnHmgetKeyTs,
    pub hmget_key: FnHmgetKey,
    pub hmput_default: FnHmputDefault,
    pub hmput_key: FnHmputKey,
    pub shmode_func: FnShmodeFunc,
    pub hmdel_key: FnHmdelKey,
    pub stralloc: FnStralloc,
    pub strreset: FnStrreset,
    pub strkey: FnStrkey,
    pub arr_push: FnArrPush,
}

pub const SYMBOL_NAMES: &[&str] = &[
    "stbds_arrgrowf",
    "stbds_arrfreef",
    "stbds_rand_seed",
    "stbds_hash_string",
    "stbds_hash_bytes",
    "stbds_hmfree_func",
    "stbds_hmget_key_ts",
    "stbds_hmget_key",
    "stbds_hmput_default",
    "stbds_hmput_key",
    "stbds_shmode_func",
    "stbds_hmdel_key",
    "stbds_stralloc",
    "stbds_strreset",
    "strkey",
    "arr_push",
];

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

/// `target/<profile>/libarr_push_lib.so`, derived from the test binary's own
/// location so it works for both `dev` and `release` test profiles.
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .join("libarr_push_lib.so")
}

macro_rules! grab {
    ($lib:expr, $ty:ty, $name:literal) => {{
        let s: libloading::Symbol<$ty> = $lib
            .get(concat!($name, "\0").as_bytes())
            .unwrap_or_else(|e| panic!("missing symbol {}: {}", $name, e));
        *s.into_raw()
    }};
}

unsafe fn load(path: &std::path::Path, name: &'static str) -> Lib {
    let lib: &'static libloading::Library = Box::leak(Box::new(
        libloading::Library::new(path)
            .unwrap_or_else(|e| panic!("dlopen {}: {}", path.display(), e)),
    ));
    Lib {
        name,
        arrgrowf: grab!(lib, FnArrGrowf, "stbds_arrgrowf"),
        arrfreef: grab!(lib, FnArrFreef, "stbds_arrfreef"),
        rand_seed: grab!(lib, FnRandSeed, "stbds_rand_seed"),
        hash_string: grab!(lib, FnHashString, "stbds_hash_string"),
        hash_bytes: grab!(lib, FnHashBytes, "stbds_hash_bytes"),
        hmfree_func: grab!(lib, FnHmfreeFunc, "stbds_hmfree_func"),
        hmget_key_ts: grab!(lib, FnHmgetKeyTs, "stbds_hmget_key_ts"),
        hmget_key: grab!(lib, FnHmgetKey, "stbds_hmget_key"),
        hmput_default: grab!(lib, FnHmputDefault, "stbds_hmput_default"),
        hmput_key: grab!(lib, FnHmputKey, "stbds_hmput_key"),
        shmode_func: grab!(lib, FnShmodeFunc, "stbds_shmode_func"),
        hmdel_key: grab!(lib, FnHmdelKey, "stbds_hmdel_key"),
        stralloc: grab!(lib, FnStralloc, "stbds_stralloc"),
        strreset: grab!(lib, FnStrreset, "stbds_strreset"),
        strkey: grab!(lib, FnStrkey, "strkey"),
        arr_push: grab!(lib, FnArrPush, "arr_push"),
    }
}

/// `target/release/libarr_push_lib.so` — the *shipping* artifact.
///
/// The C library is built with neither `-DNDEBUG` nor `-O` and therefore has no
/// sanitiser-style instrumentation.  The Rust **release** profile matches that:
/// no `debug_assertions`, so no MIR null-pointer-dereference checks.  The Rust
/// **dev** profile, in contrast, deliberately traps raw-pointer UB
/// (`"null pointer dereference occurred"` -> non-unwinding panic -> `SIGABRT`)
/// where the C simply faults (`SIGSEGV`).  Tests that intentionally drive the
/// library into UB must therefore compare against the release build.
pub fn rust_release_so_path() -> PathBuf {
    manifest_dir().join("target/release/libarr_push_lib.so")
}

pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
}

static PAIR: OnceLock<Pair> = OnceLock::new();
static PAIR_RELEASE: OnceLock<Pair> = OnceLock::new();
/// The libraries own a mutable global (`stbds_hash_seed`), so every test must
/// run exclusively.
static LOCK: Mutex<()> = Mutex::new(());

/// Rebuild the cdylib for `profile` before it is `dlopen`ed.
///
/// This is NOT optional. Because `[lib] crate-type = ["cdylib"]` produces no
/// rlib, an integration test has nothing to link against, so cargo leaves the
/// `lib` target OUT of the test unit graph: neither `cargo test` nor
/// `cargo test --test <name>` rebuilds `libarr_push_lib.so`. Without this, a
/// change to `src/lib.rs` would be silently tested against a STALE `.so` and the
/// whole suite would be meaningless. Verified by
/// `errors_fatal.rs::harness_tests_the_current_source`.
fn ensure_built(release: bool) {
    let mut cmd = std::process::Command::new(env!("CARGO"));
    cmd.args(["build", "--lib"]);
    if release {
        cmd.arg("--release");
    }
    let out = cmd
        .current_dir(manifest_dir())
        .output()
        .unwrap_or_else(|e| panic!("could not spawn cargo to rebuild the cdylib: {e}"));
    assert!(
        out.status.success(),
        "cargo build --lib{} failed, so the .so under test would be stale:\n{}",
        if release { " --release" } else { "" },
        String::from_utf8_lossy(&out.stderr)
    );
}

pub fn libs() -> &'static Pair {
    PAIR.get_or_init(|| {
        ensure_built(false);
        unsafe {
            Pair {
                c: load(&c_so_path(), "C"),
                rs: load(&rust_so_path(), "Rust"),
            }
        }
    })
}

/// The C `.so` paired with the **release** Rust `.so` (see
/// [`rust_release_so_path`]). Builds the release cdylib on demand.
pub fn libs_release() -> &'static Pair {
    PAIR_RELEASE.get_or_init(|| {
        ensure_built(true);
        let path = rust_release_so_path();
        assert!(
            path.exists(),
            "missing {} -- run `cargo build --release`",
            path.display()
        );
        unsafe {
            Pair {
                c: load(&c_so_path(), "C"),
                rs: load(&path, "Rust(release)"),
            }
        }
    })
}

/// Like [`session`] but pairs the C `.so` with the **release** Rust `.so`.
pub fn session_release(seed: usize) -> (&'static Pair, Session) {
    let p = libs_release();
    let depth = SESSION_DEPTH.with(|d| d.get());
    let g = if depth == 0 {
        Some(LOCK.lock().unwrap_or_else(|e| e.into_inner()))
    } else {
        None
    };
    SESSION_DEPTH.with(|d| d.set(depth + 1));
    unsafe {
        (p.c.rand_seed)(seed);
        (p.rs.rand_seed)(seed);
    }
    (p, Session(g))
}

std::thread_local! {
    /// Re-entrancy depth for `session()` on this thread.
    static SESSION_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Guard returned by [`session`]. Re-entrant: nesting two sessions on the same
/// thread is allowed (the inner one simply re-seeds), which keeps a test from
/// deadlocking against itself.
pub struct Session(Option<MutexGuard<'static, ()>>);

impl Drop for Session {
    fn drop(&mut self) {
        SESSION_DEPTH.with(|d| d.set(d.get() - 1));
        // self.0's MutexGuard (if any) is dropped right after this body
    }
}

/// Acquire exclusive access and reset BOTH global hash seeds to `seed` so the
/// two implementations start from an identical state.
pub fn session(seed: usize) -> (&'static Pair, Session) {
    let p = libs();
    let depth = SESSION_DEPTH.with(|d| d.get());
    let g = if depth == 0 {
        Some(LOCK.lock().unwrap_or_else(|e| e.into_inner()))
    } else {
        None
    };
    SESSION_DEPTH.with(|d| d.set(depth + 1));
    unsafe {
        (p.c.rand_seed)(seed);
        (p.rs.rand_seed)(seed);
    }
    (p, Session(g))
}

/// The library's own initial value of `stbds_hash_seed`.
pub const INITIAL_HASH_SEED: usize = 0x3141_5926;

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) — fixed seeds everywhere for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn u32(&mut self) -> u32 {
        self.u64() as u32
    }
    pub fn u8(&mut self) -> u8 {
        self.u64() as u8
    }
    /// Uniform-ish in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        (self.u64() % (n as u64)) as usize
    }
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.u8()).collect()
    }
    /// NUL-terminated printable-ASCII C string of `len` characters.
    pub fn cstring(&mut self, len: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..len).map(|_| 0x21 + (self.u64() % 0x5e) as u8).collect();
        v.push(0);
        v
    }
    /// NUL-terminated C string of `len` bytes drawn from `1..=255` (so it can
    /// contain bytes >= 0x80).
    pub fn cstring_hibytes(&mut self, len: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..len).map(|_| 1 + (self.u64() % 255) as u8).collect();
        v.push(0);
        v
    }
}

// ---------------------------------------------------------------------------
// Struct layout constants (mirrors of the C structs — must not drift)
// ---------------------------------------------------------------------------

pub const HDR_SIZE: usize = 32;
pub const HDR_LENGTH: usize = 0;
pub const HDR_CAPACITY: usize = 8;
pub const HDR_HASH_TABLE: usize = 16;
pub const HDR_TEMP: usize = 24;

pub const HI_SIZE: usize = 104;
pub const HI_TEMP_KEY: usize = 0;
pub const HI_SLOT_COUNT: usize = 8;
pub const HI_USED_COUNT: usize = 16;
pub const HI_USED_COUNT_THRESHOLD: usize = 24;
pub const HI_USED_COUNT_SHRINK_THRESHOLD: usize = 32;
pub const HI_TOMBSTONE_COUNT: usize = 40;
pub const HI_TOMBSTONE_COUNT_THRESHOLD: usize = 48;
pub const HI_SEED: usize = 56;
pub const HI_SLOT_COUNT_LOG2: usize = 64;
pub const HI_STRING: usize = 72; // stbds_string_arena
pub const HI_STORAGE: usize = 96;

pub const ARENA_SIZE: usize = 24;
pub const ARENA_STORAGE: usize = 0;
pub const ARENA_REMAINING: usize = 8;
pub const ARENA_BLOCK: usize = 16;
pub const ARENA_MODE: usize = 17;

pub const BUCKET_SIZE: usize = 128;
pub const BUCKET_LENGTH: usize = 8;

pub const STBDS_HM_BINARY: i32 = 0;
pub const STBDS_HM_STRING: i32 = 1;

pub const STBDS_SH_NONE: u8 = 0;
pub const STBDS_SH_DEFAULT: u8 = 1;
pub const STBDS_SH_STRDUP: u8 = 2;
pub const STBDS_SH_ARENA: u8 = 3;

pub const CACHE_LINE: usize = 64;

pub fn align_fwd(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

// ---------------------------------------------------------------------------
// Raw memory accessors
// ---------------------------------------------------------------------------

#[inline]
pub unsafe fn rd_usize(p: *const u8, off: usize) -> usize {
    (p.add(off) as *const usize).read_unaligned()
}
#[inline]
pub unsafe fn rd_isize(p: *const u8, off: usize) -> isize {
    (p.add(off) as *const isize).read_unaligned()
}
#[inline]
pub unsafe fn rd_ptr(p: *const u8, off: usize) -> *mut u8 {
    (p.add(off) as *const *mut u8).read_unaligned()
}
#[inline]
pub unsafe fn rd_u8(p: *const u8, off: usize) -> u8 {
    p.add(off).read()
}
#[inline]
pub unsafe fn wr_usize(p: *mut u8, off: usize, v: usize) {
    (p.add(off) as *mut usize).write_unaligned(v)
}
#[inline]
pub unsafe fn wr_ptr(p: *mut u8, off: usize, v: *mut u8) {
    (p.add(off) as *mut *mut u8).write_unaligned(v)
}
#[inline]
pub unsafe fn wr_u8(p: *mut u8, off: usize, v: u8) {
    p.add(off).write(v)
}

/// Read a NUL-terminated C string into a `Vec<u8>` (without the NUL).
pub unsafe fn cstr_bytes(p: *const u8) -> Vec<u8> {
    let mut v = Vec::new();
    let mut q = p;
    while *q != 0 {
        v.push(*q);
        q = q.add(1);
    }
    v
}

// ---------------------------------------------------------------------------
// Snapshotting — turns an opaque stb_ds array/map into comparable bytes
// ---------------------------------------------------------------------------

/// How the key field of an element should be interpreted when snapshotting.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyRepr {
    /// Element bytes are compared verbatim.
    Raw,
    /// `*(char**)(elem + keyoffset)` is a pointer; the pointed-to string is
    /// compared instead of the pointer value, and the pointer's 8 bytes are
    /// masked out of the raw element comparison.
    StrPtr { keyoffset: usize },
}

pub struct Snap(pub Vec<u8>);

impl Snap {
    fn new() -> Self {
        Snap(Vec::with_capacity(512))
    }
    fn tag(&mut self, s: &str) {
        self.0.push(b'|');
        self.0.extend_from_slice(s.as_bytes());
        self.0.push(b'=');
    }
    fn usz(&mut self, s: &str, v: usize) {
        self.tag(s);
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn isz(&mut self, s: &str, v: isize) {
        self.tag(s);
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u8v(&mut self, s: &str, v: u8) {
        self.tag(s);
        self.0.push(v);
    }
    fn blob(&mut self, s: &str, v: &[u8]) {
        self.tag(s);
        self.0.extend_from_slice(&v.len().to_le_bytes());
        self.0.extend_from_slice(v);
    }
}

/// Snapshot of a *raw array* pointer (`a`, i.e. the pointer `stbds_arrgrowf`
/// returns). `NULL` is allowed.
pub unsafe fn snap_arr(a: *mut c_void, elemsize: usize) -> Snap {
    let mut s = Snap::new();
    if a.is_null() {
        s.tag("NULL");
        return s;
    }
    let a = a as *mut u8;
    let h = a.sub(HDR_SIZE);
    let len = rd_usize(h, HDR_LENGTH);
    let cap = rd_usize(h, HDR_CAPACITY);
    s.usz("len", len);
    s.usz("cap", cap);
    s.usz("has_table", !rd_ptr(h, HDR_HASH_TABLE).is_null() as usize);
    s.isz("temp", rd_isize(h, HDR_TEMP));
    s.blob("elems", std::slice::from_raw_parts(a, len * elemsize));
    s
}

/// Snapshot of a *map* pointer (`t`, i.e. what `hmput_key`/`hmget_key` return,
/// which is `raw_array + elemsize`).
///
/// Covers: array header, every element (with string keys dereferenced), the
/// whole `stbds_hash_index` (all counters, thresholds, seed, string arena) and
/// every hash bucket's `hash[]`/`index[]`.
pub unsafe fn snap_map(t: *mut c_void, elemsize: usize, key: KeyRepr) -> Snap {
    let mut s = Snap::new();
    if t.is_null() {
        s.tag("NULL");
        return s;
    }
    let t = t as *mut u8;
    let a = t.sub(elemsize);
    let h = a.sub(HDR_SIZE);
    let len = rd_usize(h, HDR_LENGTH);
    let cap = rd_usize(h, HDR_CAPACITY);
    s.usz("len", len);
    s.usz("cap", cap);
    s.isz("temp", rd_isize(h, HDR_TEMP));

    // elements
    for i in 0..len {
        let e = a.add(i * elemsize);
        match key {
            KeyRepr::Raw => s.blob("e", std::slice::from_raw_parts(e, elemsize)),
            KeyRepr::StrPtr { keyoffset } => {
                // element 0 is the "default" slot; its key pointer is whatever
                // memset(0) left there.
                let kp = rd_ptr(e, keyoffset);
                if kp.is_null() {
                    s.tag("k_null");
                } else if i == 0 {
                    s.tag("k_slot0");
                } else {
                    s.blob("k", &cstr_bytes(kp));
                }
                let mut rest: Vec<u8> = std::slice::from_raw_parts(e, elemsize).to_vec();
                for b in rest.iter_mut().skip(keyoffset).take(8) {
                    *b = 0;
                }
                s.blob("e", &rest);
            }
        }
    }

    let tbl = rd_ptr(h, HDR_HASH_TABLE);
    if tbl.is_null() {
        s.tag("no_table");
        return s;
    }
    s.tag("table");
    s.usz("slot_count", rd_usize(tbl, HI_SLOT_COUNT));
    s.usz("used_count", rd_usize(tbl, HI_USED_COUNT));
    s.usz("uct", rd_usize(tbl, HI_USED_COUNT_THRESHOLD));
    s.usz("ucst", rd_usize(tbl, HI_USED_COUNT_SHRINK_THRESHOLD));
    s.usz("tc", rd_usize(tbl, HI_TOMBSTONE_COUNT));
    s.usz("tct", rd_usize(tbl, HI_TOMBSTONE_COUNT_THRESHOLD));
    s.usz("seed", rd_usize(tbl, HI_SEED));
    s.usz("log2", rd_usize(tbl, HI_SLOT_COUNT_LOG2));
    // storage must be the 64-byte-aligned address right after the header
    let storage = rd_ptr(tbl, HI_STORAGE);
    s.usz(
        "storage_ok",
        (storage as usize == align_fwd(tbl as usize + HI_SIZE, CACHE_LINE)) as usize,
    );
    // string arena
    let arena = tbl.add(HI_STRING);
    s.usz(
        "arena_blocks",
        arena_chain_len(rd_ptr(arena, ARENA_STORAGE)),
    );
    s.usz("arena_remaining", rd_usize(arena, ARENA_REMAINING));
    s.u8v("arena_block", rd_u8(arena, ARENA_BLOCK));
    s.u8v("arena_mode", rd_u8(arena, ARENA_MODE));
    // NOTE: `stbds_hash_index::temp_key` is deliberately NOT part of the
    // snapshot.  `stbds_make_hash_index` `realloc`s the index and memsets only
    // the `string` sub-struct, so `temp_key` holds *indeterminate heap bytes*
    // until one of the three string-mode branches of `stbds_hmput_key`
    // (lib.c:733/786/787/788) writes it.  In binary mode it is never written at
    // all, so comparing it would compare uninitialised memory.  Tests that need
    // it check it explicitly right after a put, via `Map::temp_key()`.

    // buckets
    let slot_count = rd_usize(tbl, HI_SLOT_COUNT);
    for b in 0..(slot_count >> 3) {
        let bp = storage.add(b * BUCKET_SIZE);
        for j in 0..BUCKET_LENGTH {
            s.0.extend_from_slice(&rd_usize(bp, j * 8).to_le_bytes());
        }
        for j in 0..BUCKET_LENGTH {
            s.0
                .extend_from_slice(&rd_isize(bp, 64 + j * 8).to_le_bytes());
        }
    }
    s
}

unsafe fn arena_chain_len(mut blk: *mut u8) -> usize {
    let mut n = 0usize;
    while !blk.is_null() && n < 1_000_000 {
        n += 1;
        blk = rd_ptr(blk, 0);
    }
    n
}

/// Snapshot of a bare `stbds_string_arena` plus a pointer returned by
/// `stbds_stralloc` (compared as "string contents + offset inside its block").
pub unsafe fn snap_arena(arena: *const u8, ret: *const u8) -> Snap {
    let mut s = Snap::new();
    let storage = rd_ptr(arena, ARENA_STORAGE);
    s.usz("blocks", arena_chain_len(storage));
    s.usz("remaining", rd_usize(arena, ARENA_REMAINING));
    s.u8v("block", rd_u8(arena, ARENA_BLOCK));
    s.u8v("mode", rd_u8(arena, ARENA_MODE));
    if ret.is_null() {
        s.tag("ret_null");
        return s;
    }
    s.blob("ret_str", &cstr_bytes(ret));
    // Where does the returned pointer live?  Both properties below are exact
    // (no address arithmetic on unrelated blocks), so they are deterministic
    // across processes:
    //
    //  * fast path / new-block path: `p = storage->storage + remaining - len`
    //    and `remaining -= len`, hence `p == head->storage + remaining_after`.
    //  * oversize path: returns `sb->storage`, i.e. offset 0 of the block at
    //    chain index 0 (when `storage` was NULL) or 1 (spliced after the head).
    let remaining = rd_usize(arena, ARENA_REMAINING);
    let at_head_plus_remaining = !storage.is_null()
        && ret as usize == storage.add(8) as usize + remaining;
    s.usz("ret_at_head_plus_remaining", at_head_plus_remaining as usize);
    let mut blk = storage;
    let mut idx = 0usize;
    let mut block_start: isize = -1;
    while !blk.is_null() && idx < 1_000_000 {
        if ret as usize == blk.add(8) as usize {
            block_start = idx as isize;
            break;
        }
        blk = rd_ptr(blk, 0);
        idx += 1;
    }
    s.isz("ret_is_start_of_block", block_start);
    s
}

// ---------------------------------------------------------------------------
// Assertion helper with a readable diff
// ---------------------------------------------------------------------------

pub fn hexdump(b: &[u8]) -> String {
    let mut out = String::new();
    for (i, chunk) in b.chunks(24).enumerate() {
        out.push_str(&format!("{:06x}  ", i * 24));
        for x in chunk {
            out.push_str(&format!("{:02x} ", x));
        }
        out.push_str("  ");
        for x in chunk {
            out.push(if (0x20..0x7f).contains(x) {
                *x as char
            } else {
                '.'
            });
        }
        out.push('\n');
    }
    out
}

#[track_caller]
pub fn assert_snap_eq(c: &Snap, rs: &Snap, ctx: &str) {
    if c.0 == rs.0 {
        return;
    }
    let first = c
        .0
        .iter()
        .zip(rs.0.iter())
        .position(|(a, b)| a != b)
        .unwrap_or(c.0.len().min(rs.0.len()));
    panic!(
        "DIVERGENCE [{ctx}]\n  C len={} Rust len={} first diff at byte {first}\n\
         --- C ---\n{}\n--- Rust ---\n{}",
        c.0.len(),
        rs.0.len(),
        hexdump(&c.0[first.saturating_sub(48)..(first + 96).min(c.0.len())]),
        hexdump(&rs.0[first.saturating_sub(48)..(first + 96).min(rs.0.len())]),
    );
}

#[track_caller]
pub fn assert_eq_ctx<T: PartialEq + std::fmt::Debug>(c: T, rs: T, ctx: &str) {
    if c != rs {
        panic!("DIVERGENCE [{ctx}]\n  C    = {c:?}\n  Rust = {rs:?}");
    }
}

// ---------------------------------------------------------------------------
// Fatal-path comparison: run a closure in a forked child and report how it died
// ---------------------------------------------------------------------------

extern "C" {
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn _exit(code: i32) -> !;
    fn open(path: *const std::ffi::c_char, flags: i32, ...) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
}

/// How a forked child terminated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Exited(i32),
    Signaled(i32),
}

pub const SIGABRT: i32 = 6;
pub const SIGSEGV: i32 = 11;
pub const SIGBUS: i32 = 7;

impl Outcome {
    pub fn is_fatal_signal(self) -> bool {
        matches!(self, Outcome::Signaled(_))
    }
}

/// Run `f` in a forked child with stdout/stderr redirected to `/dev/null`
/// (`assert()` prints a file/line message that legitimately differs between the
/// C and the Rust build) and report how the child terminated.
///
/// Only ever called from single-threaded test bodies -- see the module comment
/// in `tests/errors_fatal.rs`.
pub fn fork_run<F: FnOnce()>(f: F) -> Outcome {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            let fd = open(b"/dev/null\0".as_ptr() as *const std::ffi::c_char, 1);
            if fd >= 0 {
                dup2(fd, 1);
                dup2(fd, 2);
            }
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
            _exit(if r.is_ok() { 0 } else { 101 });
        }
        let mut status: i32 = 0;
        let rc = waitpid(pid, &mut status, 0);
        assert_eq!(rc, pid, "waitpid failed");
        let sig = status & 0x7f;
        if sig != 0 {
            Outcome::Signaled(sig)
        } else {
            Outcome::Exited((status >> 8) & 0xff)
        }
    }
}

/// Run the same fatal scenario against both implementations and require the
/// SAME termination (same signal, or the same clean exit code).
#[track_caller]
pub fn assert_same_fate<F>(p: &'static Pair, ctx: &str, scenario: F)
where
    F: Fn(&'static Lib) + Copy,
{
    let c = fork_run(|| scenario(&p.c));
    let rs = fork_run(|| scenario(&p.rs));
    if c != rs {
        panic!("DIVERGENT FATE [{ctx}]\n  C    = {c:?}\n  Rust = {rs:?}");
    }
    eprintln!("  [{ctx}] both -> {c:?}");
}

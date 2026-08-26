//! Differential-test harness.
//!
//! Loads BOTH shared objects with `libloading` and calls every function through
//! its exported C symbol — the Rust side is never called directly, so the
//! `#[no_mangle]` / `extern "C"` wrappers are under test too.
//!
//!  * C   : `c_src/build/libtranslated_rust.so`
//!  * Rust: `target/{debug,release}/libintput_lib.so`
//!
//! Both libraries own a *process-global* `stbds_hash_seed`, so every test must
//! (a) run serialised and (b) re-seed both sides before an op-stream. `Pair`
//! does both.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// C data layout (must stay bit-identical to `c_src/src/lib.c`)
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
pub struct StringBlock {
    pub next: *mut StringBlock,
    pub storage: [c_char; 8],
}

/// `struct stbds_string_arena`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Arena {
    pub storage: *mut StringBlock,
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
pub const BUCKET_SHIFT: usize = 3;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HashBucket {
    pub hash: [usize; BUCKET_LEN],
    pub index: [isize; BUCKET_LEN],
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
    pub string: Arena,
    pub storage: *mut HashBucket,
}

pub const HEADER_SIZE: usize = std::mem::size_of::<ArrayHeader>();

pub const STBDS_HM_BINARY: c_int = 0;
pub const STBDS_HM_STRING: c_int = 1;

pub const STBDS_SH_NONE: c_int = 0;
pub const STBDS_SH_DEFAULT: c_int = 1;
pub const STBDS_SH_STRDUP: c_int = 2;
pub const STBDS_SH_ARENA: c_int = 3;

// ---------------------------------------------------------------------------
// FFI signatures
// ---------------------------------------------------------------------------

type FnArrGrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type FnArrFreef = unsafe extern "C" fn(*mut c_void);
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
type FnStrAlloc = unsafe extern "C" fn(*mut Arena, *mut c_char) -> *mut c_char;
type FnStrReset = unsafe extern "C" fn(*mut Arena);
type FnStrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
type FnIntPut = unsafe extern "C" fn(c_int);

/// One loaded shared object with every exported symbol resolved.
pub struct Lib {
    pub name: &'static str,
    _lib: &'static Library,
    pub arrgrowf: FnArrGrowf,
    pub arrfreef: FnArrFreef,
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
    pub intput: FnIntPut,
}

unsafe fn sym<T: Copy>(lib: &'static Library, name: &[u8]) -> T {
    let s: Symbol<T> = lib
        .get(name)
        .unwrap_or_else(|e| panic!("symbol {} missing: {e}", String::from_utf8_lossy(name)));
    *s
}

impl Lib {
    unsafe fn open(name: &'static str, path: &PathBuf) -> Lib {
        let lib: &'static Library = Box::leak(Box::new(
            Library::new(path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display())),
        ));
        Lib {
            name,
            arrgrowf: sym(lib, b"stbds_arrgrowf\0"),
            arrfreef: sym(lib, b"stbds_arrfreef\0"),
            rand_seed: sym(lib, b"stbds_rand_seed\0"),
            hash_string: sym(lib, b"stbds_hash_string\0"),
            hash_bytes: sym(lib, b"stbds_hash_bytes\0"),
            hmfree_func: sym(lib, b"stbds_hmfree_func\0"),
            hmget_key_ts: sym(lib, b"stbds_hmget_key_ts\0"),
            hmget_key: sym(lib, b"stbds_hmget_key\0"),
            hmput_default: sym(lib, b"stbds_hmput_default\0"),
            hmput_key: sym(lib, b"stbds_hmput_key\0"),
            shmode_func: sym(lib, b"stbds_shmode_func\0"),
            hmdel_key: sym(lib, b"stbds_hmdel_key\0"),
            stralloc: sym(lib, b"stbds_stralloc\0"),
            strreset: sym(lib, b"stbds_strreset\0"),
            strkey: sym(lib, b"strkey\0"),
            intput: sym(lib, b"intput\0"),
            _lib: lib,
        }
    }
}

pub fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    let p = manifest().join("c_src/build/libtranslated_rust.so");
    assert!(
        p.exists(),
        "C shared object not built: {}\nrun: cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

pub fn rust_so_path() -> PathBuf {
    // The test executable lives in target/<profile>/deps/, so prefer the .so
    // that sits next to it; fall back to debug/release.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(profile_dir) = exe.parent().and_then(|d| d.parent()) {
            let p = profile_dir.join("libintput_lib.so");
            if p.exists() {
                return p;
            }
        }
    }
    for prof in ["debug", "release"] {
        let p = manifest().join("target").join(prof).join("libintput_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("Rust cdylib not built: run `cargo build` (or `cargo build --release`) first");
}

struct Libs {
    c: Lib,
    r: Lib,
}

// The two `Lib`s hold raw fn pointers; they are only ever used while the
// process-wide `LOCK` mutex is held.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());

/// A serialised handle on both libraries.
pub struct Pair {
    pub c: &'static Lib,
    pub r: &'static Lib,
    _guard: MutexGuard<'static, ()>,
}

impl Pair {
    /// Acquire the process-wide lock and hand out both libraries.
    pub fn new() -> Pair {
        let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let libs = LIBS.get_or_init(|| unsafe {
            Libs {
                c: Lib::open("C", &c_so_path()),
                r: Lib::open("Rust", &rust_so_path()),
            }
        });
        Pair {
            c: &libs.c,
            r: &libs.r,
            _guard: guard,
        }
    }

    /// Put both libraries' global `stbds_hash_seed` into the same state.
    pub fn seed(&self, s: usize) {
        unsafe {
            (self.c.rand_seed)(s);
            (self.r.rand_seed)(s);
        }
    }

    pub fn both(&self) -> [&'static Lib; 2] {
        [self.c, self.r]
    }
}

// ---------------------------------------------------------------------------
// State snapshots
// ---------------------------------------------------------------------------

/// How to interpret an element's key field when snapshotting.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyKind {
    /// Key is stored inline (binary map): compare the raw element bytes.
    Binary,
    /// Key is a `char *` at `keyoffset`: compare the pointed-to C string plus
    /// the element bytes *outside* the pointer field (pointer values are
    /// allocation-dependent and must not be compared).
    StringPtr { keyoffset: usize },
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Snap {
    pub is_null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub has_table: bool,
    pub slot_count: usize,
    pub slot_count_log2: usize,
    pub used_count: usize,
    pub used_count_threshold: usize,
    pub used_count_shrink_threshold: usize,
    pub tombstone_count: usize,
    pub tombstone_count_threshold: usize,
    pub seed: usize,
    pub arena_remaining: usize,
    pub arena_block: u8,
    pub arena_mode: u8,
    pub arena_block_count: usize,
    pub temp_key: Option<Vec<u8>>,
    pub buckets: Vec<([usize; BUCKET_LEN], [isize; BUCKET_LEN])>,
    /// Raw element bytes (binary maps) or the non-pointer element bytes.
    pub elems: Vec<Vec<u8>>,
    /// Dereferenced key strings, `None` for a NULL key pointer.
    pub keys: Vec<Option<Vec<u8>>>,
}

pub unsafe fn cstr_bytes(p: *const c_char) -> Vec<u8> {
    let mut v = Vec::new();
    let mut q = p as *const u8;
    while *q != 0 {
        v.push(*q);
        q = q.add(1);
    }
    v
}

unsafe fn arena_block_count(a: &Arena) -> usize {
    let mut n = 0usize;
    let mut x = a.storage;
    while !x.is_null() {
        n += 1;
        x = (*x).next;
        // guard against a cycle in a forged arena
        assert!(n < 1_000_000, "arena block list too long / cyclic");
    }
    n
}

/// Snapshot a hash-map handle (the pointer callers hold, i.e. `raw + elemsize`).
///
/// `elemsize` is the map's element size, `kind` says how to read keys.
/// `read_temp_key` must be `false` unless the caller *knows* `temp_key` was just
/// written: `stbds_make_hash_index` never initialises it (it only copies
/// `string` and `seed` from the old table), so after every table create / grow /
/// shrink / rebuild it holds uninitialised `malloc` bytes. Use
/// [`MapPair::check_temp_key`] right after an inserting put instead.
pub unsafe fn snap_map(
    handle: *mut c_void,
    elemsize: usize,
    kind: KeyKind,
    read_temp_key: bool,
) -> Snap {
    if handle.is_null() {
        return Snap {
            is_null: true,
            ..Default::default()
        };
    }
    let raw = (handle as *mut u8).sub(elemsize) as *mut c_void;
    snap_arr(raw, elemsize, kind, read_temp_key)
}

/// Snapshot a raw array pointer (`stbds_arrgrowf` result).
pub unsafe fn snap_arr(
    raw: *mut c_void,
    elemsize: usize,
    kind: KeyKind,
    read_temp_key: bool,
) -> Snap {
    if raw.is_null() {
        return Snap {
            is_null: true,
            ..Default::default()
        };
    }
    let h = (raw as *mut ArrayHeader).sub(1);
    let mut s = Snap {
        is_null: false,
        length: (*h).length,
        capacity: (*h).capacity,
        temp: (*h).temp,
        ..Default::default()
    };

    let t = (*h).hash_table as *mut HashIndex;
    if !t.is_null() {
        s.has_table = true;
        s.slot_count = (*t).slot_count;
        s.slot_count_log2 = (*t).slot_count_log2;
        s.used_count = (*t).used_count;
        s.used_count_threshold = (*t).used_count_threshold;
        s.used_count_shrink_threshold = (*t).used_count_shrink_threshold;
        s.tombstone_count = (*t).tombstone_count;
        s.tombstone_count_threshold = (*t).tombstone_count_threshold;
        s.seed = (*t).seed;
        s.arena_remaining = (*t).string.remaining;
        s.arena_block = (*t).string.block;
        s.arena_mode = (*t).string.mode;
        s.arena_block_count = arena_block_count(&(*t).string);
        if read_temp_key {
            s.temp_key = if (*t).temp_key.is_null() {
                None
            } else {
                Some(cstr_bytes((*t).temp_key))
            };
        }
        let nbuckets = (*t).slot_count >> BUCKET_SHIFT;
        for i in 0..nbuckets {
            let b = (*t).storage.add(i);
            s.buckets.push(((*b).hash, (*b).index));
        }
    }

    if elemsize > 0 {
        for i in 0..(*h).length {
            let e = (raw as *mut u8).add(elemsize * i);
            match kind {
                KeyKind::Binary => {
                    s.elems.push(std::slice::from_raw_parts(e, elemsize).to_vec());
                    s.keys.push(None);
                }
                KeyKind::StringPtr { keyoffset } => {
                    // element bytes with the key pointer field blanked out
                    let mut bytes = std::slice::from_raw_parts(e, elemsize).to_vec();
                    for b in bytes
                        .iter_mut()
                        .skip(keyoffset)
                        .take(std::mem::size_of::<*mut c_char>())
                    {
                        *b = 0;
                    }
                    s.elems.push(bytes);
                    let kp = *(e.add(keyoffset) as *const *const c_char);
                    s.keys
                        .push(if kp.is_null() { None } else { Some(cstr_bytes(kp)) });
                }
            }
        }
    }
    s
}

/// Snapshot a standalone `stbds_string_arena`.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ArenaSnap {
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
    pub block_count: usize,
    pub storage_is_null: bool,
}

pub unsafe fn snap_arena(a: *const Arena) -> ArenaSnap {
    ArenaSnap {
        remaining: (*a).remaining,
        block: (*a).block,
        mode: (*a).mode,
        block_count: arena_block_count(&*a),
        storage_is_null: (*a).storage.is_null(),
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xoshiro256**), fixed seeds -> reproducible runs
// ---------------------------------------------------------------------------

pub struct Rng(pub [u64; 4]);

/// Optional global seed offset, so the whole suite can be re-run over different
/// random inputs without editing any test:
///
/// ```sh
/// STBDS_DIFF_SEED=1 cargo test        # different inputs, same coverage
/// ```
///
/// Unset (the default) means the fixed, reproducible seeds baked into the tests.
fn seed_offset() -> u64 {
    static OFF: OnceLock<u64> = OnceLock::new();
    *OFF.get_or_init(|| {
        std::env::var("STBDS_DIFF_SEED")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0)
    })
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        // splitmix64 expansion
        let mut s = seed ^ seed_offset().wrapping_mul(0xD1B5_4A32_D192_ED03);
        let mut out = [0u64; 4];
        for o in out.iter_mut() {
            s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = s;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            *o = z ^ (z >> 31);
        }
        Rng(out)
    }
    pub fn next_u64(&mut self) -> u64 {
        let s = &mut self.0;
        let result = s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = s[1] << 17;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);
        result
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next_u64() as u8).collect()
    }
    pub fn i32v(&mut self) -> i32 {
        self.next_u32() as i32
    }
}

// ---------------------------------------------------------------------------
// Crash-case runner (for assert/abort paths)
// ---------------------------------------------------------------------------

pub const CRASH_ENV: &str = "STBDS_DIFF_CRASH_CASE";
pub const SIDE_ENV: &str = "STBDS_DIFF_CRASH_SIDE";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrashResult {
    pub code: Option<i32>,
    pub signal: Option<i32>,
    /// stderr with the leading `<argv0>: ` of the glibc assert message removed.
    pub assert_msg: Option<String>,
}

/// Re-exec this test binary so it runs `case` against `side` in a child
/// process, and report how the child died.
pub fn run_crash_case(case: &str, side: &str) -> CrashResult {
    use std::os::unix::process::ExitStatusExt;

    let exe = std::env::current_exe().unwrap();
    let out = std::process::Command::new(exe)
        .args(["crash_child_runner", "--exact", "--nocapture", "--test-threads=1"])
        .env(CRASH_ENV, case)
        .env(SIDE_ENV, side)
        .output()
        .expect("failed to spawn crash child");

    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let assert_msg = stderr
        .lines()
        .find(|l| l.contains("Assertion") && l.contains("failed"))
        .map(|l| match l.find(": ") {
            // strip "<argv0>: " so the two sides are comparable
            Some(i) => l[i + 2..].to_string(),
            None => l.to_string(),
        });

    CrashResult {
        code: out.status.code(),
        signal: out.status.signal(),
        assert_msg,
    }
}

/// Read the crash case requested for this (child) process, if any.
pub fn crash_request() -> Option<(String, String)> {
    let case = std::env::var(CRASH_ENV).ok()?;
    let side = std::env::var(SIDE_ENV).unwrap_or_else(|_| "c".into());
    Some((case, side))
}

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

#[track_caller]
pub fn eq_snap(what: &str, c: &Snap, r: &Snap) {
    if c != r {
        let mut msg = format!("{what}: C/Rust map state diverged\n");
        macro_rules! f {
            ($n:ident) => {
                if c.$n != r.$n {
                    msg += &format!("  {:<30} C={:?}  Rust={:?}\n", stringify!($n), c.$n, r.$n);
                }
            };
        }
        f!(is_null);
        f!(length);
        f!(capacity);
        f!(temp);
        f!(has_table);
        f!(slot_count);
        f!(slot_count_log2);
        f!(used_count);
        f!(used_count_threshold);
        f!(used_count_shrink_threshold);
        f!(tombstone_count);
        f!(tombstone_count_threshold);
        f!(seed);
        f!(arena_remaining);
        f!(arena_block);
        f!(arena_mode);
        f!(arena_block_count);
        f!(temp_key);
        if c.buckets != r.buckets {
            msg += &format!(
                "  buckets differ:\n    C   ={:?}\n    Rust={:?}\n",
                c.buckets, r.buckets
            );
        }
        if c.elems != r.elems {
            msg += &format!(
                "  elems differ:\n    C   ={:?}\n    Rust={:?}\n",
                c.elems, r.elems
            );
        }
        if c.keys != r.keys {
            msg += &format!(
                "  keys differ:\n    C   ={:?}\n    Rust={:?}\n",
                c.keys, r.keys
            );
        }
        panic!("{msg}");
    }
}

// ---------------------------------------------------------------------------
// Parallel C/Rust map driver
// ---------------------------------------------------------------------------

/// Drives the *same* sequence of low-level `stbds_hm*` calls on both `.so`s.
///
/// This mirrors what the `stbds_hmput`/`stbds_shput`/… macros expand to in a
/// real consumer: call the exported `stbds_hm*_key` entry point, then write the
/// element at the returned `temp` index.
pub struct MapPair {
    pub elemsize: usize,
    pub keysize: usize,
    /// `mode` argument handed to every `stbds_hm*_key` call.
    pub mode: c_int,
    pub kind: KeyKind,
    pub read_temp_key: bool,
    pub hc: *mut c_void,
    pub hr: *mut c_void,
}

impl MapPair {
    /// Start from a NULL handle (what `T *m = NULL;` gives a real consumer).
    pub fn null(elemsize: usize, keysize: usize, mode: c_int, kind: KeyKind) -> MapPair {
        MapPair {
            elemsize,
            keysize,
            mode,
            kind,
            read_temp_key: false,
            hc: std::ptr::null_mut(),
            hr: std::ptr::null_mut(),
        }
    }

    /// Start from `stbds_shmode_func(elemsize, sh_mode)` (= `sh_new_arena` /
    /// `sh_new_strdup` in the header macros).
    pub fn shmode(
        p: &Pair,
        elemsize: usize,
        keysize: usize,
        mode: c_int,
        sh_mode: c_int,
        kind: KeyKind,
    ) -> MapPair {
        let (hc, hr) = unsafe {
            (
                (p.c.shmode_func)(elemsize, sh_mode),
                (p.r.shmode_func)(elemsize, sh_mode),
            )
        };
        MapPair {
            elemsize,
            keysize,
            mode,
            kind,
            read_temp_key: false,
            hc,
            hr,
            }
    }

    pub fn snaps(&self) -> (Snap, Snap) {
        unsafe {
            (
                snap_map(self.hc, self.elemsize, self.kind, self.read_temp_key),
                snap_map(self.hr, self.elemsize, self.kind, self.read_temp_key),
            )
        }
    }

    #[track_caller]
    pub fn check(&self, what: &str) {
        let (c, r) = self.snaps();
        eq_snap(what, &c, &r);
    }

    /// Write `value` into the element the last call selected (`temp`), leaving
    /// the key field untouched for string maps.
    unsafe fn write_value(&self, handle: *mut c_void, temp: isize, value: &[u8]) {
        if self.elemsize == 0 || value.is_empty() {
            return;
        }
        let off = match self.kind {
            KeyKind::Binary => self.keysize,
            KeyKind::StringPtr { .. } => std::mem::size_of::<*mut c_char>(),
        };
        if off >= self.elemsize {
            return;
        }
        // `stbds_arrgrowf` reallocs without zeroing, so element bytes the test
        // never writes are indeterminate heap residue and CANNOT be compared
        // between the two libraries. Require every test to initialise the whole
        // value region so a short `value` shows up as a test bug, not as a
        // spurious C/Rust divergence.
        assert!(
            value.len() >= self.elemsize - off,
            "MapPair value must cover the whole value region: got {} bytes, \
             need {} (elemsize={}, key region={})",
            value.len(),
            self.elemsize - off,
            self.elemsize,
            off
        );
        let n = value.len().min(self.elemsize - off);
        let e = (handle as *mut u8).add(self.elemsize * temp as usize).add(off);
        std::ptr::copy_nonoverlapping(value.as_ptr(), e, n);
    }

    /// `stbds_hmput_key` on both sides + the macro's element write.
    /// Returns the two `temp` indices (asserted equal).
    #[track_caller]
    pub fn put(&mut self, p: &Pair, key: &mut [u8], value: &[u8]) -> isize {
        let (tc, tr) = unsafe {
            self.hc = (p.c.hmput_key)(
                self.hc,
                self.elemsize,
                key.as_mut_ptr() as *mut c_void,
                self.keysize,
                self.mode,
            );
            let tc = self.temp_of(self.hc);
            self.write_value(self.hc, tc, value);

            self.hr = (p.r.hmput_key)(
                self.hr,
                self.elemsize,
                key.as_mut_ptr() as *mut c_void,
                self.keysize,
                self.mode,
            );
            let tr = self.temp_of(self.hr);
            self.write_value(self.hr, tr, value);
            (tc, tr)
        };
        assert_eq!(tc, tr, "put temp diverged (key={key:?})");
        tc
    }

    /// `stbds_hmput_key` for a string map: the library owns the key, the caller
    /// only writes `.value` (exactly what `stbds_shput` does).
    #[track_caller]
    pub fn put_str(&mut self, p: &Pair, key: *mut c_char, value: &[u8]) -> isize {
        let (tc, tr) = unsafe {
            self.hc = (p.c.hmput_key)(
                self.hc,
                self.elemsize,
                key as *mut c_void,
                self.keysize,
                self.mode,
            );
            let tc = self.temp_of(self.hc);
            self.write_value(self.hc, tc, value);

            self.hr = (p.r.hmput_key)(
                self.hr,
                self.elemsize,
                key as *mut c_void,
                self.keysize,
                self.mode,
            );
            let tr = self.temp_of(self.hr);
            self.write_value(self.hr, tr, value);
            (tc, tr)
        };
        assert_eq!(tc, tr, "put_str temp diverged");
        tc
    }

    unsafe fn temp_of(&self, handle: *mut c_void) -> isize {
        let raw = (handle as *mut u8).sub(self.elemsize) as *mut ArrayHeader;
        (*raw.sub(1)).temp
    }

    /// `stbds_hmget_key` on both sides; returns `temp` and the element bytes.
    #[track_caller]
    pub fn get(&mut self, p: &Pair, key: &mut [u8]) -> isize {
        let (tc, tr) = unsafe {
            self.hc = (p.c.hmget_key)(
                self.hc,
                self.elemsize,
                key.as_mut_ptr() as *mut c_void,
                self.keysize,
                self.mode,
            );
            self.hr = (p.r.hmget_key)(
                self.hr,
                self.elemsize,
                key.as_mut_ptr() as *mut c_void,
                self.keysize,
                self.mode,
            );
            (self.temp_of(self.hc), self.temp_of(self.hr))
        };
        assert_eq!(tc, tr, "get temp diverged (key={key:?})");
        self.assert_elem_at(tc);
        tc
    }

    #[track_caller]
    pub fn get_str(&mut self, p: &Pair, key: *mut c_char) -> isize {
        let (tc, tr) = unsafe {
            self.hc = (p.c.hmget_key)(
                self.hc,
                self.elemsize,
                key as *mut c_void,
                self.keysize,
                self.mode,
            );
            self.hr = (p.r.hmget_key)(
                self.hr,
                self.elemsize,
                key as *mut c_void,
                self.keysize,
                self.mode,
            );
            (self.temp_of(self.hc), self.temp_of(self.hr))
        };
        assert_eq!(tc, tr, "get_str temp diverged");
        self.assert_elem_at(tc);
        tc
    }

    /// `stbds_hmget_key_ts` on both sides (temp via the out-parameter).
    #[track_caller]
    pub fn get_ts(&mut self, p: &Pair, key: &mut [u8]) -> isize {
        let mut tc: isize = 0x5A5A;
        let mut tr: isize = 0x5A5A;
        unsafe {
            self.hc = (p.c.hmget_key_ts)(
                self.hc,
                self.elemsize,
                key.as_mut_ptr() as *mut c_void,
                self.keysize,
                &mut tc,
                self.mode,
            );
            self.hr = (p.r.hmget_key_ts)(
                self.hr,
                self.elemsize,
                key.as_mut_ptr() as *mut c_void,
                self.keysize,
                &mut tr,
                self.mode,
            );
        }
        assert_eq!(tc, tr, "get_ts temp diverged (key={key:?})");
        self.assert_elem_at(tc);
        tc
    }

    /// `stbds_hmdel_key` on both sides; returns the `temp` flag (1 = deleted).
    #[track_caller]
    pub fn del(&mut self, p: &Pair, key: &mut [u8], keyoffset: usize) -> isize {
        let (tc, tr) = unsafe {
            self.hc = (p.c.hmdel_key)(
                self.hc,
                self.elemsize,
                key.as_mut_ptr() as *mut c_void,
                self.keysize,
                keyoffset,
                self.mode,
            );
            self.hr = (p.r.hmdel_key)(
                self.hr,
                self.elemsize,
                key.as_mut_ptr() as *mut c_void,
                self.keysize,
                keyoffset,
                self.mode,
            );
            let tc = if self.hc.is_null() {
                0
            } else {
                self.temp_of(self.hc)
            };
            let tr = if self.hr.is_null() {
                0
            } else {
                self.temp_of(self.hr)
            };
            (tc, tr)
        };
        assert_eq!(
            self.hc.is_null(),
            self.hr.is_null(),
            "del NULL-ness diverged"
        );
        assert_eq!(tc, tr, "del temp diverged (key={key:?})");
        tc
    }

    #[track_caller]
    pub fn del_str(&mut self, p: &Pair, key: *mut c_char, keyoffset: usize) -> isize {
        let (tc, tr) = unsafe {
            self.hc = (p.c.hmdel_key)(
                self.hc,
                self.elemsize,
                key as *mut c_void,
                self.keysize,
                keyoffset,
                self.mode,
            );
            self.hr = (p.r.hmdel_key)(
                self.hr,
                self.elemsize,
                key as *mut c_void,
                self.keysize,
                keyoffset,
                self.mode,
            );
            let tc = if self.hc.is_null() { 0 } else { self.temp_of(self.hc) };
            let tr = if self.hr.is_null() { 0 } else { self.temp_of(self.hr) };
            (tc, tr)
        };
        assert_eq!(self.hc.is_null(), self.hr.is_null(), "del NULL-ness diverged");
        assert_eq!(tc, tr, "del_str temp diverged");
        tc
    }

    /// Compare the element bytes at `idx` (skipped for a miss / NULL handle).
    #[track_caller]
    fn assert_elem_at(&self, idx: isize) {
        if idx < 0 || self.elemsize == 0 || self.hc.is_null() || self.hr.is_null() {
            return;
        }
        unsafe {
            let ec = std::slice::from_raw_parts(
                (self.hc as *const u8).add(self.elemsize * idx as usize),
                self.elemsize,
            );
            let er = std::slice::from_raw_parts(
                (self.hr as *const u8).add(self.elemsize * idx as usize),
                self.elemsize,
            );
            match self.kind {
                KeyKind::Binary => assert_eq!(ec, er, "element {idx} bytes diverged"),
                KeyKind::StringPtr { keyoffset } => {
                    let psz = std::mem::size_of::<*mut c_char>();
                    assert_eq!(
                        &ec[..keyoffset],
                        &er[..keyoffset],
                        "element {idx} pre-key bytes diverged"
                    );
                    assert_eq!(
                        &ec[keyoffset + psz..],
                        &er[keyoffset + psz..],
                        "element {idx} value bytes diverged"
                    );
                    let kc = *(ec.as_ptr().add(keyoffset) as *const *const c_char);
                    let kr = *(er.as_ptr().add(keyoffset) as *const *const c_char);
                    assert_eq!(kc.is_null(), kr.is_null(), "element {idx} key NULL-ness");
                    if !kc.is_null() {
                        assert_eq!(
                            cstr_bytes(kc),
                            cstr_bytes(kr),
                            "element {idx} key string diverged"
                        );
                    }
                }
            }
        }
    }

    /// `stbds_hmfree_func` on both sides.
    pub fn free(&mut self, p: &Pair) {
        unsafe {
            if !self.hc.is_null() {
                (p.c.hmfree_func)((self.hc as *mut u8).sub(self.elemsize) as *mut c_void, self.elemsize);
            }
            if !self.hr.is_null() {
                (p.r.hmfree_func)((self.hr as *mut u8).sub(self.elemsize) as *mut c_void, self.elemsize);
            }
        }
        self.hc = std::ptr::null_mut();
        self.hr = std::ptr::null_mut();
    }
}

impl MapPair {
    /// Compare `table->temp_key` on both sides.
    ///
    /// `stbds_make_hash_index` never initialises `temp_key`, so this is only
    /// valid immediately after a put that *did* write it (an insert, or a
    /// forward-scan "found existing key" hit in string mode).
    #[track_caller]
    pub fn check_temp_key(&self, what: &str) {
        unsafe {
            let tk = |h: *mut c_void| -> Option<Vec<u8>> {
                let hdr = ((h as *mut u8).sub(self.elemsize) as *mut ArrayHeader).sub(1);
                let t = (*hdr).hash_table as *mut HashIndex;
                if t.is_null() || (*t).temp_key.is_null() {
                    None
                } else {
                    Some(cstr_bytes((*t).temp_key))
                }
            };
            let (a, b) = (tk(self.hc), tk(self.hr));
            assert_eq!(a, b, "{what}: table->temp_key diverged");
        }
    }

    /// Current `length` of the C-side array (both sides are asserted equal by
    /// `check`, so one is enough for control flow).
    pub fn length(&self) -> usize {
        if self.hc.is_null() {
            return 0;
        }
        unsafe {
            let hdr = ((self.hc as *mut u8).sub(self.elemsize) as *mut ArrayHeader).sub(1);
            (*hdr).length
        }
    }
}

//! Differential-test harness.
//!
//! Loads BOTH shared objects (the C reference and the Rust translation) with
//! `libloading` and exposes their exported symbols behind identical function
//! pointers, so every call in every test crosses a real FFI boundary and
//! exercises the `#[no_mangle]` wrappers.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// Signatures of the 16 exported symbols
// ---------------------------------------------------------------------------

pub type FnArrGrowF = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrFreeF = unsafe extern "C" fn(*mut c_void);
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnStrAlloc = unsafe extern "C" fn(*mut CArena, *mut c_char) -> *mut c_char;
pub type FnStrReset = unsafe extern "C" fn(*mut CArena);
pub type FnHmFree = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmGetKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmGetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type FnHmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmPutKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnShModeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnStrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnHmGeti = unsafe extern "C" fn(c_int);

pub struct Lib {
    pub name: &'static str,
    _lib: libloading::Library,
    pub arrgrowf: FnArrGrowF,
    pub arrfreef: FnArrFreeF,
    pub rand_seed: FnRandSeed,
    pub hash_bytes: FnHashBytes,
    pub hash_string: FnHashString,
    pub stralloc: FnStrAlloc,
    pub strreset: FnStrReset,
    pub hmfree_func: FnHmFree,
    pub hmget_key: FnHmGetKey,
    pub hmget_key_ts: FnHmGetKeyTs,
    pub hmput_default: FnHmPutDefault,
    pub hmput_key: FnHmPutKey,
    pub hmdel_key: FnHmDelKey,
    pub shmode_func: FnShModeFunc,
    pub strkey: FnStrKey,
    pub hm_geti: FnHmGeti,
}

impl Lib {
    unsafe fn open(name: &'static str, path: &PathBuf) -> Lib {
        let lib = libloading::Library::new(path)
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
        macro_rules! g {
            ($t:ty, $sym:literal) => {
                *lib
                    .get::<$t>($sym)
                    .unwrap_or_else(|e| panic!("{} missing {}: {e}", name, stringify!($sym)))
            };
        }
        Lib {
            name,
            arrgrowf: g!(FnArrGrowF, b"stbds_arrgrowf\0"),
            arrfreef: g!(FnArrFreeF, b"stbds_arrfreef\0"),
            rand_seed: g!(FnRandSeed, b"stbds_rand_seed\0"),
            hash_bytes: g!(FnHashBytes, b"stbds_hash_bytes\0"),
            hash_string: g!(FnHashString, b"stbds_hash_string\0"),
            stralloc: g!(FnStrAlloc, b"stbds_stralloc\0"),
            strreset: g!(FnStrReset, b"stbds_strreset\0"),
            hmfree_func: g!(FnHmFree, b"stbds_hmfree_func\0"),
            hmget_key: g!(FnHmGetKey, b"stbds_hmget_key\0"),
            hmget_key_ts: g!(FnHmGetKeyTs, b"stbds_hmget_key_ts\0"),
            hmput_default: g!(FnHmPutDefault, b"stbds_hmput_default\0"),
            hmput_key: g!(FnHmPutKey, b"stbds_hmput_key\0"),
            hmdel_key: g!(FnHmDelKey, b"stbds_hmdel_key\0"),
            shmode_func: g!(FnShModeFunc, b"stbds_shmode_func\0"),
            strkey: g!(FnStrKey, b"strkey\0"),
            hm_geti: g!(FnHmGeti, b"hm_geti\0"),
            _lib: lib,
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
    let build = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|e| e == "so").unwrap_or(false) {
                found.push(p);
            }
        }
    }
    found.sort();
    found.pop().unwrap_or_else(|| {
        panic!(
            "no .so found in {} — build the C library first:\n  \
             cd c_src && mkdir -p build && cd build && cmake .. \
             -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("target/release/libhm_geti_lib.so");
    assert!(
        p.exists(),
        "{} missing — run `cargo build --release` before `cargo test`",
        p.display()
    );
    p
}

pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
}

static PAIR: OnceLock<Pair> = OnceLock::new();
static SERIAL: Mutex<()> = Mutex::new(());

/// Both libraries carry a mutable global (`stbds_hash_seed`), so all tests must
/// run serially and reseed explicitly. Holding this guard guarantees that.
pub fn libs() -> (&'static Pair, MutexGuard<'static, ()>) {
    let guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let pair = PAIR.get_or_init(|| unsafe {
        Pair {
            c: Lib::open("C", &c_so_path()),
            rs: Lib::open("RUST", &rust_so_path()),
        }
    });
    (pair, guard)
}

/// Reseed both libraries' global hash seed to the same value.
pub fn reseed(p: &Pair, seed: usize) {
    unsafe {
        (p.c.rand_seed)(seed);
        (p.rs.rand_seed)(seed);
    }
}

/// The library's own compiled-in default (`static size_t stbds_hash_seed = 0x31415926`).
pub const DEFAULT_SEED: usize = 0x3141_5926;

// ---------------------------------------------------------------------------
// C data layout mirrors
// ---------------------------------------------------------------------------

pub const HEADER_SIZE: usize = 32;
pub const BUCKET_LENGTH: usize = 8;
pub const BUCKET_SHIFT: usize = 3;
pub const BUCKET_MASK: usize = BUCKET_LENGTH - 1;

pub const STBDS_HM_BINARY: c_int = 0;
pub const STBDS_HM_STRING: c_int = 1;
pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CArena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

impl CArena {
    pub fn zeroed() -> CArena {
        CArena {
            storage: std::ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        }
    }
}

#[repr(C)]
pub struct CHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
pub struct CBucket {
    pub hash: [usize; BUCKET_LENGTH],
    pub index: [isize; BUCKET_LENGTH],
}

#[repr(C)]
pub struct CHashIndex {
    pub temp_key: *mut c_char,
    pub slot_count: usize,
    pub used_count: usize,
    pub used_count_threshold: usize,
    pub used_count_shrink_threshold: usize,
    pub tombstone_count: usize,
    pub tombstone_count_threshold: usize,
    pub seed: usize,
    pub slot_count_log2: usize,
    pub string: CArena,
    pub storage: *mut CBucket,
}

const _: () = assert!(std::mem::size_of::<CHeader>() == HEADER_SIZE);
const _: () = assert!(std::mem::size_of::<CBucket>() == 128);
const _: () = assert!(std::mem::size_of::<CHashIndex>() == 104);
const _: () = assert!(std::mem::size_of::<CArena>() == 24);

/// `stbds_header(a)`
#[inline]
pub unsafe fn header(a: *mut c_void) -> *mut CHeader {
    (a as *mut u8).wrapping_sub(HEADER_SIZE) as *mut CHeader
}

/// `STBDS_HASH_TO_ARR`
#[inline]
pub fn hash_to_arr(t: *mut c_void, elemsize: usize) -> *mut c_void {
    (t as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

/// `STBDS_ARR_TO_HASH`
#[inline]
pub fn arr_to_hash(a: *mut c_void, elemsize: usize) -> *mut c_void {
    (a as *mut u8).wrapping_add(elemsize) as *mut c_void
}

// ---------------------------------------------------------------------------
// Snapshots (allocation-address independent)
// ---------------------------------------------------------------------------

/// How to canonicalise the key portion of each element so that pointer values
/// (which necessarily differ between the two independently-allocated libraries)
/// do not cause false mismatches.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyKind {
    /// key is inline bytes — compare the raw element bytes
    Bytes,
    /// key field at offset 0 is a `char *` — compare the pointed-to C string
    /// plus the remaining element bytes
    Ptr,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct IdxSnap {
    pub slot_count: usize,
    pub used_count: usize,
    pub used_count_threshold: usize,
    pub used_count_shrink_threshold: usize,
    pub tombstone_count: usize,
    pub tombstone_count_threshold: usize,
    pub seed: usize,
    pub slot_count_log2: usize,
    pub string_remaining: usize,
    pub string_block: u8,
    pub string_mode: u8,
    pub string_storage_null: bool,
    /// (hash, index) for every slot, in slot order
    pub slots: Vec<(usize, isize)>,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct ArrSnap {
    pub is_null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub idx: Option<IdxSnap>,
    /// canonicalised element payload
    pub elems: Vec<Vec<u8>>,
}

unsafe fn cstr_bytes(p: *const c_char) -> Vec<u8> {
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

unsafe fn idx_snap(t: *mut CHashIndex) -> IdxSnap {
    let n = (*t).slot_count;
    let mut slots = Vec::with_capacity(n);
    let nb = n >> BUCKET_SHIFT;
    for i in 0..nb {
        let b = (*t).storage.add(i);
        for j in 0..BUCKET_LENGTH {
            slots.push(((*b).hash[j], (*b).index[j]));
        }
    }
    IdxSnap {
        slot_count: (*t).slot_count,
        used_count: (*t).used_count,
        used_count_threshold: (*t).used_count_threshold,
        used_count_shrink_threshold: (*t).used_count_shrink_threshold,
        tombstone_count: (*t).tombstone_count,
        tombstone_count_threshold: (*t).tombstone_count_threshold,
        seed: (*t).seed,
        slot_count_log2: (*t).slot_count_log2,
        string_remaining: (*t).string.remaining,
        string_block: (*t).string.block,
        string_mode: (*t).string.mode,
        string_storage_null: (*t).string.storage.is_null(),
        slots,
    }
}

/// Snapshot the raw array pointer `a` (i.e. `arr`, NOT the hash-biased pointer).
pub unsafe fn snap_arr(a: *mut c_void, elemsize: usize, kind: KeyKind) -> ArrSnap {
    if a.is_null() {
        return ArrSnap {
            is_null: true,
            length: 0,
            capacity: 0,
            temp: 0,
            idx: None,
            elems: Vec::new(),
        };
    }
    let h = header(a);
    let ht = (*h).hash_table as *mut CHashIndex;
    let idx = if ht.is_null() {
        None
    } else {
        Some(idx_snap(ht))
    };
    let mut elems = Vec::new();
    for i in 0..(*h).length {
        let e = (a as *mut u8).add(elemsize * i);
        match kind {
            KeyKind::Bytes => {
                elems.push(std::slice::from_raw_parts(e, elemsize).to_vec());
            }
            KeyKind::Ptr => {
                // element 0 is the zeroed "default" slot: its key pointer is
                // NULL, so never dereference it.
                let kp = *(e as *const *const c_char);
                let mut v = if i == 0 && kp.is_null() {
                    b"<default>".to_vec()
                } else {
                    cstr_bytes(kp)
                };
                v.push(0xFF);
                if elemsize > 8 {
                    v.extend_from_slice(std::slice::from_raw_parts(e.add(8), elemsize - 8));
                }
                elems.push(v);
            }
        }
    }
    ArrSnap {
        is_null: false,
        length: (*h).length,
        capacity: (*h).capacity,
        temp: (*h).temp,
        idx,
        elems,
    }
}

/// Snapshot from the hash-biased pointer returned by the `stbds_hm*` functions.
pub unsafe fn snap_hm(t: *mut c_void, elemsize: usize, kind: KeyKind) -> ArrSnap {
    if t.is_null() {
        return snap_arr(std::ptr::null_mut(), elemsize, kind);
    }
    snap_arr(hash_to_arr(t, elemsize), elemsize, kind)
}

/// `stbds_temp_key(arr)` — only valid after a string-mode put.
pub unsafe fn temp_key(t: *mut c_void, elemsize: usize) -> Vec<u8> {
    let a = hash_to_arr(t, elemsize);
    let ht = (*header(a)).hash_table as *mut CHashIndex;
    if ht.is_null() {
        return b"<no-table>".to_vec();
    }
    cstr_bytes((*ht).temp_key)
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds keep every run reproducible
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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// uniform in `0..n`
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next_u32() as u8).collect()
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

/// Leak a NUL-terminated copy of `s`, zero-padded by 64 bytes.
///
/// Padding matters: in `SH_NONE` / `default:` mode `stbds_hmput_key` `memcpy`s
/// `keysize` bytes *out of the key string*, which can read past the NUL. Zero
/// padding makes those reads deterministic so C and Rust stay comparable.
///
/// String keys stored in `SH_DEFAULT` mode are the caller's own pointers, so
/// they must also outlive the map — hence the leak.
pub fn leak_cstr(s: &str) -> *mut c_char {
    let b = s.as_bytes();
    assert!(!b.contains(&0), "test key must not contain NUL");
    let mut v: Vec<u8> = Vec::with_capacity(b.len() + 64);
    v.extend_from_slice(b);
    v.resize(b.len() + 64, 0);
    Box::leak(v.into_boxed_slice()).as_mut_ptr() as *mut c_char
}

// ---------------------------------------------------------------------------
// Child-process execution, for comparing abort/assert behaviour
// ---------------------------------------------------------------------------

extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    fn write(fd: c_int, buf: *const c_void, n: usize) -> isize;
    fn close(fd: c_int) -> c_int;
}

/// Outcome of running a closure in a forked child.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Outcome {
    Exited(i32),
    Signalled(i32),
}

fn decode_status(status: c_int) -> Outcome {
    if status & 0x7f == 0x7f {
        Outcome::Signalled((status >> 8) & 0xff)
    } else if status & 0x7f == 0 {
        Outcome::Exited((status >> 8) & 0xff)
    } else {
        Outcome::Signalled(status & 0x7f)
    }
}

/// Run `f` in a forked child and report how the child terminated. Used for the
/// `assert()` rows of `ERRORS.md` (which abort the process in both builds) and
/// for the rows whose C behaviour corrupts the heap or faults.
///
/// A panic inside `f` — e.g. a failed `assert_eq!` comparing the two libraries —
/// is caught and turned into exit status 101, so it can never unwind back into
/// the parent's test harness from inside the child.
pub fn run_in_child<F: FnOnce()>(f: F) -> Outcome {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            disable_core_dumps();
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
            _exit(if r.is_ok() { 0 } else { 101 });
        }
        let mut status: c_int = 0;
        let r = waitpid(pid, &mut status, 0);
        assert_eq!(r, pid, "waitpid failed");
        decode_status(status)
    }
}

/// Several `ERRORS.md` rows are expected to fault. Without this, every such child
/// hands a core dump to the system core handler, which dominates the wall clock.
fn disable_core_dumps() {
    unsafe {
        let rl = RLimit { cur: 0, max: 0 };
        setrlimit(RLIMIT_CORE, &rl);
    }
}

/// Like [`run_in_child`], but the child also sends a byte string back over a
/// pipe. This is how observations are collected from scenarios that go on to
/// crash or corrupt the heap: capture first, then let the child die.
pub fn run_in_child_capture<F: FnOnce() -> Vec<u8>>(f: F) -> (Outcome, Vec<u8>) {
    unsafe {
        let mut fds = [0 as c_int; 2];
        assert_eq!(pipe(fds.as_mut_ptr()), 0, "pipe failed");
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            close(fds[0]);
            disable_core_dumps();
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
            match r {
                Ok(v) => {
                    let mut off = 0usize;
                    while off < v.len() {
                        let w = write(
                            fds[1],
                            v.as_ptr().add(off) as *const c_void,
                            v.len() - off,
                        );
                        if w <= 0 {
                            break;
                        }
                        off += w as usize;
                    }
                    close(fds[1]);
                    _exit(0);
                }
                Err(_) => {
                    close(fds[1]);
                    _exit(101);
                }
            }
        }
        close(fds[1]);
        let mut out = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = read(fds[0], buf.as_mut_ptr() as *mut c_void, buf.len());
            if n <= 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
        close(fds[0]);
        let mut status: c_int = 0;
        assert_eq!(waitpid(pid, &mut status, 0), pid, "waitpid failed");
        (decode_status(status), out)
    }
}

/// Like [`run_in_child_capture`], but the child first caps its address space
/// (`RLIMIT_AS`). Some `ERRORS.md` rows drive the C into requesting multi-gigabyte
/// blocks; without a cap glibc spends over a second failing each one. The cap is
/// applied identically to both libraries, so the comparison stays valid while the
/// allocation-failure path is reached immediately.
pub fn run_in_child_capture_limited<F: FnOnce() -> Vec<u8>>(
    limit_bytes: u64,
    f: F,
) -> (Outcome, Vec<u8>) {
    run_in_child_capture(move || {
        unsafe {
            let rl = RLimit {
                cur: limit_bytes,
                max: limit_bytes,
            };
            setrlimit(RLIMIT_AS, &rl);
        }
        f()
    })
}

#[repr(C)]
struct RLimit {
    cur: u64,
    max: u64,
}
/// `RLIMIT_AS` / `RLIMIT_CORE` on Linux
const RLIMIT_AS: c_int = 9;
const RLIMIT_CORE: c_int = 4;

extern "C" {
    fn setrlimit(resource: c_int, rlim: *const RLimit) -> c_int;
}

/// Address-space-capped variant of [`assert_same_capture`].
#[track_caller]
pub fn assert_same_capture_limited<F: Fn(&Lib) -> Vec<u8> + Copy>(
    p: &Pair,
    limit_bytes: u64,
    what: &str,
    f: F,
) -> (Outcome, Vec<u8>) {
    let (oc, vc) = run_in_child_capture_limited(limit_bytes, || f(&p.c));
    let (or, vr) = run_in_child_capture_limited(limit_bytes, || f(&p.rs));
    assert_eq!(oc, or, "[{what}] termination differs: C={oc:?} RS={or:?}");
    assert_eq!(
        vc, vr,
        "[{what}] observations differ:\n C={vc:02x?}\nRS={vr:02x?}"
    );
    (oc, vc)
}

/// Run the same scenario against the C `.so` and the Rust `.so`, each in its own
/// forked child, and require identical termination (both clean, or both killed
/// by the same signal — e.g. `SIGABRT` from a live `assert`).
#[track_caller]
pub fn assert_same_outcome<F: Fn(&Lib) + Copy>(p: &Pair, what: &str, f: F) -> Outcome {
    let oc = run_in_child(|| f(&p.c));
    let or = run_in_child(|| f(&p.rs));
    assert_eq!(oc, or, "[{what}] termination differs: C={oc:?} RS={or:?}");
    oc
}

/// Same, but also require the captured observations to be byte-identical.
#[track_caller]
pub fn assert_same_capture<F: Fn(&Lib) -> Vec<u8> + Copy>(
    p: &Pair,
    what: &str,
    f: F,
) -> (Outcome, Vec<u8>) {
    let (oc, vc) = run_in_child_capture(|| f(&p.c));
    let (or, vr) = run_in_child_capture(|| f(&p.rs));
    assert_eq!(oc, or, "[{what}] termination differs: C={oc:?} RS={or:?}");
    assert_eq!(
        vc, vr,
        "[{what}] observations differ:\n C={vc:02x?}\nRS={vr:02x?}"
    );
    (oc, vc)
}

// ---------------------------------------------------------------------------
// DualMap — drives the same hash-map op stream through both .so files
// ---------------------------------------------------------------------------

/// A pair of hash maps, one per library, kept in lockstep.
///
/// `t*` are the *hash-biased* pointers the `stbds_hm*` functions return and
/// consume, exactly as the `stbds_hmput`/`shput` macros keep them.
pub struct DualMap<'a> {
    pub p: &'a Pair,
    pub elemsize: usize,
    pub keysize: usize,
    pub kind: KeyKind,
    pub tc: *mut c_void,
    pub tr: *mut c_void,
    pub ctx: String,
}

impl<'a> DualMap<'a> {
    /// Start from `NULL` (the way a user declares `T *map = NULL;`).
    pub fn empty(p: &'a Pair, elemsize: usize, keysize: usize, kind: KeyKind, ctx: &str) -> Self {
        DualMap {
            p,
            elemsize,
            keysize,
            kind,
            tc: std::ptr::null_mut(),
            tr: std::ptr::null_mut(),
            ctx: ctx.to_string(),
        }
    }

    /// Start from `stbds_shmode_func(elemsize, mode)` (i.e. `sh_new_strdup` /
    /// `sh_new_arena`, or any other raw mode value).
    pub fn shmode(
        p: &'a Pair,
        elemsize: usize,
        keysize: usize,
        kind: KeyKind,
        mode: c_int,
        ctx: &str,
    ) -> Self {
        unsafe {
            DualMap {
                p,
                elemsize,
                keysize,
                kind,
                tc: (p.c.shmode_func)(elemsize, mode),
                tr: (p.rs.shmode_func)(elemsize, mode),
                ctx: ctx.to_string(),
            }
        }
    }

    pub fn snaps(&self) -> (ArrSnap, ArrSnap) {
        unsafe {
            (
                snap_hm(self.tc, self.elemsize, self.kind),
                snap_hm(self.tr, self.elemsize, self.kind),
            )
        }
    }

    #[track_caller]
    pub fn assert_same(&self, what: &str) {
        let (sc, sr) = self.snaps();
        if sc != sr {
            panic!(
                "[{}] state divergence after {what}\n C: {:#?}\nRS: {:#?}",
                self.ctx, sc, sr
            );
        }
    }

    /// `stbds_hmput`: call `stbds_hmput_key`, then write key+value at `temp`
    /// exactly as the macro does.
    #[track_caller]
    pub fn put_bytes(&mut self, key: &[u8], value: &[u8]) {
        assert_eq!(key.len(), self.keysize);
        assert_eq!(value.len(), self.elemsize - self.keysize);
        unsafe {
            let mut kc = key.to_vec();
            let mut kr = key.to_vec();
            self.tc = (self.p.c.hmput_key)(
                self.tc,
                self.elemsize,
                kc.as_mut_ptr() as *mut c_void,
                self.keysize,
                STBDS_HM_BINARY,
            );
            self.tr = (self.p.rs.hmput_key)(
                self.tr,
                self.elemsize,
                kr.as_mut_ptr() as *mut c_void,
                self.keysize,
                STBDS_HM_BINARY,
            );
            let ic = temp_of(self.tc, self.elemsize);
            let ir = temp_of(self.tr, self.elemsize);
            assert_eq!(ic, ir, "[{}] hmput_key temp mismatch", self.ctx);
            write_elem(self.tc, self.elemsize, ic, key, value);
            write_elem(self.tr, self.elemsize, ir, key, value);
        }
        self.assert_same("put_bytes");
    }

    /// `stbds_shput`-alike: string-mode put with an explicit `mode` value.
    /// Returns the published `temp` index.
    #[track_caller]
    pub fn put_str(&mut self, key: *mut c_char, value: &[u8], mode: c_int) -> isize {
        assert_eq!(value.len(), self.elemsize - self.keysize);
        unsafe {
            self.tc = (self.p.c.hmput_key)(
                self.tc,
                self.elemsize,
                key as *mut c_void,
                self.keysize,
                mode,
            );
            self.tr = (self.p.rs.hmput_key)(
                self.tr,
                self.elemsize,
                key as *mut c_void,
                self.keysize,
                mode,
            );
            let ic = temp_of(self.tc, self.elemsize);
            let ir = temp_of(self.tr, self.elemsize);
            assert_eq!(ic, ir, "[{}] hmput_key(str) temp mismatch", self.ctx);
            // only the value half; the key field belongs to the library
            write_value(self.tc, self.elemsize, self.keysize, ic, value);
            write_value(self.tr, self.elemsize, self.keysize, ir, value);
            self.assert_same("put_str");
            ic
        }
    }

    /// `stbds_hmgeti`: `stbds_hmget_key` + read `stbds_temp`.
    #[track_caller]
    pub fn geti(&mut self, key: &[u8], mode: c_int) -> isize {
        unsafe {
            let mut kc = key.to_vec();
            let mut kr = key.to_vec();
            self.tc = (self.p.c.hmget_key)(
                self.tc,
                self.elemsize,
                kc.as_mut_ptr() as *mut c_void,
                self.keysize,
                mode,
            );
            self.tr = (self.p.rs.hmget_key)(
                self.tr,
                self.elemsize,
                kr.as_mut_ptr() as *mut c_void,
                self.keysize,
                mode,
            );
            let ic = temp_of(self.tc, self.elemsize);
            let ir = temp_of(self.tr, self.elemsize);
            assert_eq!(ic, ir, "[{}] hmgeti mismatch for {key:?}", self.ctx);
            self.assert_same("geti");
            ic
        }
    }

    /// `stbds_shgeti`
    #[track_caller]
    pub fn geti_str(&mut self, key: *mut c_char, mode: c_int) -> isize {
        unsafe {
            self.tc = (self.p.c.hmget_key)(
                self.tc,
                self.elemsize,
                key as *mut c_void,
                self.keysize,
                mode,
            );
            self.tr = (self.p.rs.hmget_key)(
                self.tr,
                self.elemsize,
                key as *mut c_void,
                self.keysize,
                mode,
            );
            let ic = temp_of(self.tc, self.elemsize);
            let ir = temp_of(self.tr, self.elemsize);
            assert_eq!(ic, ir, "[{}] shgeti mismatch", self.ctx);
            self.assert_same("geti_str");
            ic
        }
    }

    /// `stbds_hmgeti_ts`
    #[track_caller]
    pub fn geti_ts(&mut self, key: &[u8], mode: c_int) -> isize {
        unsafe {
            let mut kc = key.to_vec();
            let mut kr = key.to_vec();
            let mut ic: isize = 0x5A5A;
            let mut ir: isize = 0x5A5A;
            self.tc = (self.p.c.hmget_key_ts)(
                self.tc,
                self.elemsize,
                kc.as_mut_ptr() as *mut c_void,
                self.keysize,
                &mut ic,
                mode,
            );
            self.tr = (self.p.rs.hmget_key_ts)(
                self.tr,
                self.elemsize,
                kr.as_mut_ptr() as *mut c_void,
                self.keysize,
                &mut ir,
                mode,
            );
            assert_eq!(ic, ir, "[{}] hmgeti_ts mismatch for {key:?}", self.ctx);
            self.assert_same("geti_ts");
            ic
        }
    }

    /// `stbds_hmdel`: returns the value the macro yields
    /// (`t ? stbds_temp(t-1) : 0`).
    #[track_caller]
    pub fn del_bytes(&mut self, key: &[u8], keyoffset: usize, mode: c_int) -> isize {
        unsafe {
            let mut kc = key.to_vec();
            let mut kr = key.to_vec();
            self.tc = (self.p.c.hmdel_key)(
                self.tc,
                self.elemsize,
                kc.as_mut_ptr() as *mut c_void,
                self.keysize,
                keyoffset,
                mode,
            );
            self.tr = (self.p.rs.hmdel_key)(
                self.tr,
                self.elemsize,
                kr.as_mut_ptr() as *mut c_void,
                self.keysize,
                keyoffset,
                mode,
            );
            let rc = if self.tc.is_null() {
                0
            } else {
                temp_of(self.tc, self.elemsize)
            };
            let rr = if self.tr.is_null() {
                0
            } else {
                temp_of(self.tr, self.elemsize)
            };
            assert_eq!(rc, rr, "[{}] hmdel mismatch for {key:?}", self.ctx);
            self.assert_same("del_bytes");
            rc
        }
    }

    /// `stbds_shdel`
    #[track_caller]
    pub fn del_str(&mut self, key: *mut c_char, keyoffset: usize, mode: c_int) -> isize {
        unsafe {
            self.tc = (self.p.c.hmdel_key)(
                self.tc,
                self.elemsize,
                key as *mut c_void,
                self.keysize,
                keyoffset,
                mode,
            );
            self.tr = (self.p.rs.hmdel_key)(
                self.tr,
                self.elemsize,
                key as *mut c_void,
                self.keysize,
                keyoffset,
                mode,
            );
            let rc = if self.tc.is_null() {
                0
            } else {
                temp_of(self.tc, self.elemsize)
            };
            let rr = if self.tr.is_null() {
                0
            } else {
                temp_of(self.tr, self.elemsize)
            };
            assert_eq!(rc, rr, "[{}] shdel mismatch", self.ctx);
            self.assert_same("del_str");
            rc
        }
    }

    /// `stbds_hmdefault`
    #[track_caller]
    pub fn put_default(&mut self, value: &[u8]) {
        unsafe {
            self.tc = (self.p.c.hmput_default)(self.tc, self.elemsize);
            self.tr = (self.p.rs.hmput_default)(self.tr, self.elemsize);
            // (t)[-1].value = v
            write_value(self.tc, self.elemsize, self.keysize, -1, value);
            write_value(self.tr, self.elemsize, self.keysize, -1, value);
        }
        self.assert_same("put_default");
    }

    /// `stbds_hmfree`
    pub fn free(&mut self) {
        unsafe {
            if !self.tc.is_null() {
                (self.p.c.hmfree_func)(hash_to_arr(self.tc, self.elemsize), self.elemsize);
            }
            if !self.tr.is_null() {
                (self.p.rs.hmfree_func)(hash_to_arr(self.tr, self.elemsize), self.elemsize);
            }
            self.tc = std::ptr::null_mut();
            self.tr = std::ptr::null_mut();
        }
    }

    /// `stbds_temp_key` contents (string modes only).
    #[track_caller]
    pub fn assert_temp_key_same(&self) {
        unsafe {
            let a = temp_key(self.tc, self.elemsize);
            let b = temp_key(self.tr, self.elemsize);
            assert_eq!(
                a,
                b,
                "[{}] temp_key mismatch: {:?} vs {:?}",
                self.ctx,
                String::from_utf8_lossy(&a),
                String::from_utf8_lossy(&b)
            );
        }
    }
}

/// `stbds_temp((t)-1)` for a hash-biased pointer.
pub unsafe fn temp_of(t: *mut c_void, elemsize: usize) -> isize {
    (*header(hash_to_arr(t, elemsize))).temp
}

/// Write both halves of element `i` (hash-biased index), as the
/// `stbds_hmput` macro does.
unsafe fn write_elem(t: *mut c_void, elemsize: usize, i: isize, key: &[u8], value: &[u8]) {
    let e = (t as *mut u8).offset(i * elemsize as isize);
    std::ptr::copy_nonoverlapping(key.as_ptr(), e, key.len());
    std::ptr::copy_nonoverlapping(value.as_ptr(), e.add(key.len()), value.len());
}

/// Write only the value half of element `i` (hash-biased index).
unsafe fn write_value(t: *mut c_void, elemsize: usize, keysize: usize, i: isize, value: &[u8]) {
    assert_eq!(value.len(), elemsize - keysize);
    let e = (t as *mut u8).offset(i * elemsize as isize);
    std::ptr::copy_nonoverlapping(value.as_ptr(), e.add(keysize), value.len());
}

// ---------------------------------------------------------------------------
// SoloMap — drives ONE library, for cases where the C aborts (assert) and the
// comparison has to happen on process exit status instead of in memory
// ---------------------------------------------------------------------------

pub struct SoloMap<'a> {
    pub lib: &'a Lib,
    pub elemsize: usize,
    pub keysize: usize,
    pub t: *mut c_void,
}

impl<'a> SoloMap<'a> {
    pub fn empty(lib: &'a Lib, elemsize: usize, keysize: usize) -> Self {
        SoloMap {
            lib,
            elemsize,
            keysize,
            t: std::ptr::null_mut(),
        }
    }

    pub fn shmode(lib: &'a Lib, elemsize: usize, keysize: usize, mode: c_int) -> Self {
        SoloMap {
            lib,
            elemsize,
            keysize,
            t: unsafe { (lib.shmode_func)(elemsize, mode) },
        }
    }

    pub unsafe fn put_bytes(&mut self, key: &[u8], value: &[u8]) -> isize {
        let mut k = key.to_vec();
        self.t = (self.lib.hmput_key)(
            self.t,
            self.elemsize,
            k.as_mut_ptr() as *mut c_void,
            self.keysize,
            STBDS_HM_BINARY,
        );
        let i = temp_of(self.t, self.elemsize);
        write_elem(self.t, self.elemsize, i, key, value);
        i
    }

    pub unsafe fn put_str(&mut self, key: *mut c_char, value: &[u8], mode: c_int) -> isize {
        self.t = (self.lib.hmput_key)(
            self.t,
            self.elemsize,
            key as *mut c_void,
            self.keysize,
            mode,
        );
        let i = temp_of(self.t, self.elemsize);
        write_value(self.t, self.elemsize, self.keysize, i, value);
        i
    }

    pub unsafe fn geti(&mut self, key: &[u8], mode: c_int) -> isize {
        let mut k = key.to_vec();
        self.t = (self.lib.hmget_key)(
            self.t,
            self.elemsize,
            k.as_mut_ptr() as *mut c_void,
            self.keysize,
            mode,
        );
        temp_of(self.t, self.elemsize)
    }

    pub unsafe fn geti_str(&mut self, key: *mut c_char, mode: c_int) -> isize {
        self.t = (self.lib.hmget_key)(
            self.t,
            self.elemsize,
            key as *mut c_void,
            self.keysize,
            mode,
        );
        temp_of(self.t, self.elemsize)
    }

    pub unsafe fn del_bytes(&mut self, key: &[u8], keyoffset: usize, mode: c_int) -> isize {
        let mut k = key.to_vec();
        self.t = (self.lib.hmdel_key)(
            self.t,
            self.elemsize,
            k.as_mut_ptr() as *mut c_void,
            self.keysize,
            keyoffset,
            mode,
        );
        if self.t.is_null() {
            0
        } else {
            temp_of(self.t, self.elemsize)
        }
    }

    pub unsafe fn del_str(&mut self, key: *mut c_char, keyoffset: usize, mode: c_int) -> isize {
        self.t = (self.lib.hmdel_key)(
            self.t,
            self.elemsize,
            key as *mut c_void,
            self.keysize,
            keyoffset,
            mode,
        );
        if self.t.is_null() {
            0
        } else {
            temp_of(self.t, self.elemsize)
        }
    }

    pub unsafe fn put_default(&mut self, value: &[u8]) {
        self.t = (self.lib.hmput_default)(self.t, self.elemsize);
        write_value(self.t, self.elemsize, self.keysize, -1, value);
    }

    pub unsafe fn free(&mut self) {
        if !self.t.is_null() {
            (self.lib.hmfree_func)(hash_to_arr(self.t, self.elemsize), self.elemsize);
            self.t = std::ptr::null_mut();
        }
    }
}

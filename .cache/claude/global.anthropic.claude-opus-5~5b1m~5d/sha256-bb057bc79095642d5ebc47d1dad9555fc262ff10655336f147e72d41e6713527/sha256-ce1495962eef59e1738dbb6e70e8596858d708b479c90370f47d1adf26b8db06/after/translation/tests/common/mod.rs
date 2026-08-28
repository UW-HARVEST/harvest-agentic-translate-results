//! Shared differential-test harness.
//!
//! Loads BOTH the original C `.so` and the translated Rust `.so` with
//! `libloading` and calls every entry point through the FFI boundary, so the
//! `#[no_mangle]` export wrappers are exercised exactly as an external consumer
//! would exercise them.
//!
//! Nothing in this module ever calls a Rust function of the crate directly.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits the harness itself needs
// ---------------------------------------------------------------------------

extern "C" {
    pub fn malloc(n: usize) -> *mut c_void;
    pub fn free(p: *mut c_void);
    pub fn strlen(s: *const c_char) -> usize;
    pub fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    pub fn fflush(f: *mut c_void) -> c_int;
    pub fn dup(fd: c_int) -> c_int;
    pub fn dup2(old: c_int, new: c_int) -> c_int;
    pub fn close(fd: c_int) -> c_int;
    pub fn fork() -> c_int;
    pub fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    pub fn _exit(code: c_int) -> !;
}

// ---------------------------------------------------------------------------
// C-ABI mirrors of the library's internal structures (verified against a C
// sizeof/offsetof probe: 32 / 16 / 24 / 128 / 104, string@72, storage@96)
// ---------------------------------------------------------------------------

pub const BUCKET_LEN: usize = 8;
pub const BUCKET_SHIFT: usize = 3;
pub const BUCKET_MASK: usize = 7;

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;

pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

pub const HEADER_SIZE: usize = std::mem::size_of::<ArrayHeader>();

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StringArena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

impl StringArena {
    pub fn new() -> StringArena {
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
    pub string: StringArena,
    pub storage: *mut HashBucket,
}

#[inline]
pub fn header_of(arr: *mut c_void) -> *mut ArrayHeader {
    ((arr as usize).wrapping_sub(HEADER_SIZE)) as *mut ArrayHeader
}

/// `STBDS_HASH_TO_ARR`
#[inline]
pub fn hash_to_arr(t: *mut c_void, elemsize: usize) -> *mut c_void {
    ((t as usize).wrapping_sub(elemsize)) as *mut c_void
}

/// `STBDS_ARR_TO_HASH`
#[inline]
pub fn arr_to_hash(a: *mut c_void, elemsize: usize) -> *mut c_void {
    ((a as usize).wrapping_add(elemsize)) as *mut c_void
}

// ---------------------------------------------------------------------------
// Symbol table of one library
// ---------------------------------------------------------------------------

pub type FnArrGrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrFreef = unsafe extern "C" fn(*mut c_void);
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHmfreeFunc = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmgetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type FnHmgetKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmputDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmputKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnShmodeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnHmdelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnStralloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
pub type FnStrreset = unsafe extern "C" fn(*mut StringArena);
pub type FnStrkey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnStrPut = unsafe extern "C" fn(c_int);

/// The 16 exported symbols, resolved out of one shared object.
pub struct Api {
    pub name: &'static str,
    pub path: PathBuf,
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
    pub str_put: FnStrPut,
}

macro_rules! sym {
    ($lib:expr, $ty:ty, $name:literal) => {{
        let s: libloading::Symbol<$ty> = unsafe { $lib.get($name).expect(concat!(
            "missing exported symbol ", stringify!($name))) };
        unsafe { *s.into_raw() }
    }};
}

impl Api {
    fn load(name: &'static str, path: PathBuf) -> Api {
        // Leaked on purpose: the resolved function pointers must stay valid for
        // the whole process lifetime.
        let lib: &'static libloading::Library = Box::leak(Box::new(unsafe {
            libloading::Library::new(&path)
                .unwrap_or_else(|e| panic!("cannot dlopen {}: {e}", path.display()))
        }));
        Api {
            name,
            path,
            arrgrowf: sym!(lib, FnArrGrowf, b"stbds_arrgrowf\0"),
            arrfreef: sym!(lib, FnArrFreef, b"stbds_arrfreef\0"),
            rand_seed: sym!(lib, FnRandSeed, b"stbds_rand_seed\0"),
            hash_string: sym!(lib, FnHashString, b"stbds_hash_string\0"),
            hash_bytes: sym!(lib, FnHashBytes, b"stbds_hash_bytes\0"),
            hmfree_func: sym!(lib, FnHmfreeFunc, b"stbds_hmfree_func\0"),
            hmget_key_ts: sym!(lib, FnHmgetKeyTs, b"stbds_hmget_key_ts\0"),
            hmget_key: sym!(lib, FnHmgetKey, b"stbds_hmget_key\0"),
            hmput_default: sym!(lib, FnHmputDefault, b"stbds_hmput_default\0"),
            hmput_key: sym!(lib, FnHmputKey, b"stbds_hmput_key\0"),
            shmode_func: sym!(lib, FnShmodeFunc, b"stbds_shmode_func\0"),
            hmdel_key: sym!(lib, FnHmdelKey, b"stbds_hmdel_key\0"),
            stralloc: sym!(lib, FnStralloc, b"stbds_stralloc\0"),
            strreset: sym!(lib, FnStrreset, b"stbds_strreset\0"),
            strkey: sym!(lib, FnStrkey, b"strkey\0"),
            str_put: sym!(lib, FnStrPut, b"str_put\0"),
        }
    }
}

pub struct Libs {
    pub c: Api,
    pub rs: Api,
}

static LIBS: OnceLock<Libs> = OnceLock::new();
/// Both libraries carry mutable process-global state (`stbds_hash_seed`, the
/// `strkey` static buffer), so tests must not run concurrently.
static LOCK: Mutex<()> = Mutex::new(());

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    let build = manifest_dir().join("..").join("c_src").join("build");
    if let Ok(rd) = std::fs::read_dir(&build) {
        let mut hits: Vec<PathBuf> = rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension().map(|e| e == "so").unwrap_or(false)
                    && p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with("lib"))
                        .unwrap_or(false)
            })
            .collect();
        hits.sort();
        if let Some(p) = hits.pop() {
            return p;
        }
    }
    // Not built yet: build it.
    build_c_so();
    let rd = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("no {} after cmake build: {e}", build.display()));
    let mut hits: Vec<PathBuf> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "so").unwrap_or(false))
        .collect();
    hits.sort();
    hits.pop().expect("cmake produced no .so")
}

fn build_c_so() {
    let src = manifest_dir().join("..").join("c_src");
    let build = src.join("build");
    std::fs::create_dir_all(&build).expect("mkdir c_src/build");
    let ok = std::process::Command::new("cmake")
        .arg("..")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .current_dir(&build)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    assert!(ok, "cmake configure failed");
    let ok = std::process::Command::new("cmake")
        .arg("--build")
        .arg(".")
        .current_dir(&build)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    assert!(ok, "cmake build failed");
}

fn find_rust_so() -> PathBuf {
    // .../target/<profile>/deps/<testbin>  ->  .../target/<profile>/libstr_put_lib.so
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .to_path_buf();
    let p = profile_dir.join("libstr_put_lib.so");
    assert!(
        p.exists(),
        "rust cdylib not found at {} (run `cargo build` first)",
        p.display()
    );
    p
}

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| Libs {
        c: Api::load("C", find_c_so()),
        rs: Api::load("RUST", find_rust_so()),
    })
}

/// Serialises tests and resets both libraries' global hash seed so the two
/// implementations always start from an identical state.
pub fn with_libs<R>(seed: usize, f: impl FnOnce(&'static Api, &'static Api) -> R) -> R {
    let l = libs();
    let _g: MutexGuard<'_, ()> = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        (l.c.rand_seed)(seed);
        (l.rs.rand_seed)(seed);
    }
    f(&l.c, &l.rs)
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) -- fixed seeds, reproducible
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0xC0FF_EE00_1234_5678)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_usize(&mut self) -> usize {
        self.next_u64() as usize
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % (n as u64)) as usize
        }
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| (self.next_u64() >> 24) as u8).collect()
    }
    /// A random NUL-terminated ASCII string of `len` visible characters.
    pub fn cstring(&mut self, len: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..len)
            .map(|_| b'!' + (self.below(93) as u8))
            .collect();
        v.push(0);
        v
    }
    /// A random NUL-terminated string whose bytes may have the high bit set.
    pub fn cstring_high(&mut self, len: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..len)
            .map(|_| {
                let b = (self.next_u64() >> 24) as u8;
                if b == 0 {
                    0x80
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
// Snapshots: address-independent, byte-exact views of library state
// ---------------------------------------------------------------------------

/// How the first 8 bytes of an element should be canonicalised.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyKind {
    /// Raw key bytes of `keysize` at offset 0; whole element is comparable.
    Binary,
    /// Offset 0 holds a `char *`; the two libraries were handed the *same*
    /// pointer, so compare it verbatim.
    PtrSameAddr,
    /// Offset 0 holds a `char *` the library allocated itself; compare the
    /// pointed-to string content instead of the address.
    PtrByContent,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BucketSnap {
    pub hash: Vec<usize>,
    pub index: Vec<isize>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TableSnap {
    /// NOTE: `stbds_hash_index::temp_key` is deliberately **absent**.
    /// `stbds_make_hash_index` never initialises it (not even when rehashing
    /// from an old table), so it holds `realloc` garbage until the first
    /// `hmput_key` in a pointer-key mode writes it. Dereferencing it from the
    /// harness would fault. It is instead compared *relationally* in
    /// `MapPair::put` (see `temp_key_rel`), which is exactly what the
    /// `stbds_shputs` macro observes.
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

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MapSnap {
    pub is_null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub has_table: bool,
    /// One canonical byte-blob per array element `0..length`.
    pub elems: Vec<Vec<u8>>,
    pub table: Option<TableSnap>,
}

unsafe fn read_cstr(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        return None;
    }
    let n = strlen(p);
    Some(std::slice::from_raw_parts(p as *const u8, n).to_vec())
}

/// Canonicalise one element: replaces the leading `char *` with the string it
/// points at when `kind == PtrByContent`.
unsafe fn canon_elem(e: *const u8, elemsize: usize, kind: KeyKind, is_default_elem: bool) -> Vec<u8> {
    let raw = std::slice::from_raw_parts(e, elemsize);
    if is_default_elem || kind != KeyKind::PtrByContent || elemsize < 8 {
        return raw.to_vec();
    }
    let mut out = Vec::with_capacity(elemsize + 16);
    let ptr = *(e as *const *const c_char);
    match read_cstr(ptr) {
        None => out.extend_from_slice(b"<null-key>"),
        Some(s) => {
            out.extend_from_slice(b"<key:");
            out.extend_from_slice(&s);
            out.push(b'>');
        }
    }
    out.extend_from_slice(&raw[8..]);
    out
}

/// Snapshot a hash-map given the *hash-space* pointer `t` returned by the
/// library (`NULL` is fine).
pub unsafe fn snap_map(t: *mut c_void, elemsize: usize, kind: KeyKind) -> MapSnap {
    if t.is_null() {
        return MapSnap {
            is_null: true,
            length: 0,
            capacity: 0,
            temp: 0,
            has_table: false,
            elems: Vec::new(),
            table: None,
        };
    }
    let arr = hash_to_arr(t, elemsize);
    let h = &*header_of(arr);
    let table = if h.hash_table.is_null() {
        None
    } else {
        Some(snap_table(h.hash_table as *const HashIndex))
    };
    let mut elems = Vec::with_capacity(h.length);
    if elemsize > 0 {
        for i in 0..h.length {
            let e = (arr as *const u8).add(elemsize * i);
            elems.push(canon_elem(e, elemsize, kind, i == 0));
        }
    }
    MapSnap {
        is_null: false,
        length: h.length,
        capacity: h.capacity,
        temp: h.temp,
        has_table: table.is_some(),
        elems,
        table,
    }
}

pub unsafe fn snap_table(ti: *const HashIndex) -> TableSnap {
    let t = &*ti;
    let nbuckets = t.slot_count >> BUCKET_SHIFT;
    let mut buckets = Vec::with_capacity(nbuckets);
    for i in 0..nbuckets {
        let b = &*t.storage.add(i);
        buckets.push(BucketSnap {
            hash: b.hash.to_vec(),
            index: b.index.to_vec(),
        });
    }
    TableSnap {
        slot_count: t.slot_count,
        used_count: t.used_count,
        used_count_threshold: t.used_count_threshold,
        used_count_shrink_threshold: t.used_count_shrink_threshold,
        tombstone_count: t.tombstone_count,
        tombstone_count_threshold: t.tombstone_count_threshold,
        seed: t.seed,
        slot_count_log2: t.slot_count_log2,
        arena_remaining: t.string.remaining,
        arena_block: t.string.block,
        arena_mode: t.string.mode,
        arena_has_storage: !t.string.storage.is_null(),
        buckets,
    }
}

/// Snapshot of a plain dynamic array (no hash table): header + `length` elements.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ArrSnap {
    pub is_null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub has_table: bool,
    pub elems: Vec<u8>,
}

pub unsafe fn snap_arr(a: *mut c_void, elemsize: usize) -> ArrSnap {
    if a.is_null() {
        return ArrSnap {
            is_null: true,
            length: 0,
            capacity: 0,
            temp: 0,
            has_table: false,
            elems: Vec::new(),
        };
    }
    let h = &*header_of(a);
    ArrSnap {
        is_null: false,
        length: h.length,
        capacity: h.capacity,
        temp: h.temp,
        has_table: !h.hash_table.is_null(),
        elems: if elemsize == 0 {
            Vec::new()
        } else {
            std::slice::from_raw_parts(a as *const u8, elemsize * h.length).to_vec()
        },
    }
}

/// Snapshot of a string arena: the pointer itself differs between libraries, so
/// the block chain is described by length + per-block payload.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ArenaSnap {
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
    pub chain_len: usize,
}

pub unsafe fn snap_arena(a: *const StringArena) -> ArenaSnap {
    let s = &*a;
    let mut chain_len = 0usize;
    let mut p = s.storage as *const *const c_void; // first field is `next`
    while !p.is_null() {
        chain_len += 1;
        if chain_len > 1_000_000 {
            panic!("arena chain cycle");
        }
        p = *p as *const *const c_void;
    }
    ArenaSnap {
        remaining: s.remaining,
        block: s.block,
        mode: s.mode,
        chain_len,
    }
}

// ---------------------------------------------------------------------------
// stdout capture (for `str_put`)
// ---------------------------------------------------------------------------

pub fn capture_stdout(f: impl FnOnce()) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::io::AsRawFd;

    let mut tmp = tempfile();
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(tmp.as_raw_fd(), 1) >= 0, "dup2 failed");
        f();
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
    }
    let mut out = Vec::new();
    tmp.seek(SeekFrom::Start(0)).unwrap();
    tmp.read_to_end(&mut out).unwrap();
    out
}

fn tempfile() -> std::fs::File {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    let path = PathBuf::from(dir).join(format!(
        "strput_capture_{}_{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("open temp capture file");
    let _ = std::fs::remove_file(&path); // unlink; fd stays valid
    f
}

// ---------------------------------------------------------------------------
// Subprocess helper for cases where the C aborts / faults
// ---------------------------------------------------------------------------

/// Outcome of running a snippet in a forked child.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ChildOutcome {
    /// `true` = exited normally, `false` = killed by a signal
    pub exited: bool,
    /// exit status when `exited`, otherwise the signal number
    pub code: i32,
    /// everything the child wrote to stderr (glibc's `assert` diagnostic)
    pub stderr: Vec<u8>,
}

/// Runs `f` in a forked child, capturing stderr, and reports how it terminated.
///
/// Used for the rows where the C library legitimately dies: a failing
/// `STBDS_ASSERT` (SIGABRT plus a glibc diagnostic) or a dereference of a
/// wild pointer (SIGSEGV). The Rust translation must die in exactly the same
/// way, with the same message.
pub fn run_child_out(f: impl FnOnce()) -> ChildOutcome {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::io::AsRawFd;
    let mut tmp = tempfile();
    unsafe {
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            dup2(tmp.as_raw_fd(), 2);
            dup2(tmp.as_raw_fd(), 1);
            f();
            fflush(std::ptr::null_mut());
            _exit(0);
        }
        let mut status: c_int = 0;
        let r = waitpid(pid, &mut status, 0);
        assert_eq!(r, pid, "waitpid failed");
        let (exited, code) = if status & 0x7f == 0 {
            (true, (status >> 8) & 0xff)
        } else {
            (false, status & 0x7f)
        };
        let mut err = Vec::new();
        tmp.seek(SeekFrom::Start(0)).unwrap();
        tmp.read_to_end(&mut err).unwrap();
        ChildOutcome {
            exited,
            code,
            stderr: err,
        }
    }
}

pub const SIGABRT: i32 = 6;
pub const SIGSEGV: i32 = 11;
pub const SIGBUS: i32 = 7;

/// Runs the same operation against each library in its own child process and
/// asserts they terminate identically (same signal / exit code *and* the same
/// stderr diagnostic). Returns the shared outcome.
#[track_caller]
pub fn assert_same_crash(
    what: &str,
    cf: impl FnOnce(),
    rf: impl FnOnce(),
) -> ChildOutcome {
    let a = run_child_out(cf);
    let b = run_child_out(rf);
    if a != b {
        panic!(
            "DIVERGENCE in {what}
  C   = exited={} code={} stderr={:?}
  RUST= exited={} code={} stderr={:?}",
            a.exited,
            a.code,
            String::from_utf8_lossy(&a.stderr),
            b.exited,
            b.code,
            String::from_utf8_lossy(&b.stderr),
        );
    }
    a
}

/// Runs `f` in a forked child with stderr silenced and returns
/// `(exited_normally, exit_code_or_signal)`.
pub fn run_in_child(f: impl FnOnce()) -> (bool, i32) {
    unsafe {
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // child
            let devnull = std::fs::OpenOptions::new()
                .write(true)
                .open("/dev/null")
                .ok();
            if let Some(d) = devnull {
                use std::os::unix::io::AsRawFd;
                dup2(d.as_raw_fd(), 2);
            }
            f();
            fflush(std::ptr::null_mut());
            _exit(0);
        }
        let mut status: c_int = 0;
        let r = waitpid(pid, &mut status, 0);
        assert_eq!(r, pid, "waitpid failed");
        // WIFEXITED / WEXITSTATUS / WTERMSIG
        if status & 0x7f == 0x7f {
            (false, (status >> 8) & 0xff) // stopped -- shouldn't happen
        } else if status & 0x7f == 0 {
            (true, (status >> 8) & 0xff)
        } else {
            (false, status & 0x7f)
        }
    }
}

// ---------------------------------------------------------------------------
// Small helpers for driving the map API the way `stb_ds.h`'s macros do
// ---------------------------------------------------------------------------

/// `stbds_temp(t-1)` -- the index the last put/get resolved to.
pub unsafe fn map_temp(t: *mut c_void, elemsize: usize) -> isize {
    (*header_of(hash_to_arr(t, elemsize))).temp
}

/// `stbds_temp_key(t-1)` -- `*(char **) header->hash_table`
pub unsafe fn map_temp_key(t: *mut c_void, elemsize: usize) -> *mut c_char {
    let ht = (*header_of(hash_to_arr(t, elemsize))).hash_table;
    if ht.is_null() {
        std::ptr::null_mut()
    } else {
        *(ht as *mut *mut c_char)
    }
}

/// The live `stbds_hash_index` behind a map (NULL when there is none).
pub unsafe fn table_of(t: *mut c_void, elemsize: usize) -> *mut HashIndex {
    if t.is_null() {
        return std::ptr::null_mut();
    }
    (*header_of(hash_to_arr(t, elemsize))).hash_table as *mut HashIndex
}

/// Finds the bucket slot (absolute slot number) whose stored `index` equals
/// `want`, or `None`.
pub unsafe fn find_slot_with_index(ti: *mut HashIndex, want: isize) -> Option<usize> {
    let n = (*ti).slot_count >> BUCKET_SHIFT;
    for b in 0..n {
        let bucket = &*(*ti).storage.add(b);
        for i in 0..BUCKET_LEN {
            if bucket.index[i] == want {
                return Some(b * BUCKET_LEN + i);
            }
        }
    }
    None
}

/// Finds the bucket slot holding `hash`, or `None`.
pub unsafe fn find_slot_with_hash(ti: *mut HashIndex, hash: usize) -> Option<usize> {
    let n = (*ti).slot_count >> BUCKET_SHIFT;
    for b in 0..n {
        let bucket = &*(*ti).storage.add(b);
        for i in 0..BUCKET_LEN {
            if bucket.hash[i] == hash {
                return Some(b * BUCKET_LEN + i);
            }
        }
    }
    None
}

pub unsafe fn slot_index(ti: *mut HashIndex, slot: usize) -> isize {
    (*(*ti).storage.add(slot >> BUCKET_SHIFT)).index[slot & BUCKET_MASK]
}

pub unsafe fn set_slot_index(ti: *mut HashIndex, slot: usize, v: isize) {
    (*(*ti).storage.add(slot >> BUCKET_SHIFT)).index[slot & BUCKET_MASK] = v;
}

/// `stbds_hmlen(t)` -- `header(t-1)->length - 1`
pub unsafe fn map_len(t: *mut c_void, elemsize: usize) -> isize {
    if t.is_null() {
        0
    } else {
        ((*header_of(hash_to_arr(t, elemsize))).length as isize).wrapping_sub(1)
    }
}

/// Writes `payload` over the bytes of hash-space element `idx`, leaving the
/// first `skip` bytes (the key) alone. Emulates what a real `hmput` macro does
/// after the library call (`t[temp].value = v`).
pub unsafe fn write_elem_tail(t: *mut c_void, elemsize: usize, idx: isize, skip: usize, payload: &[u8]) {
    if elemsize <= skip {
        return;
    }
    let dst = (t as *mut u8).offset(idx * elemsize as isize).add(skip);
    let n = std::cmp::min(payload.len(), elemsize - skip);
    std::ptr::copy_nonoverlapping(payload.as_ptr(), dst, n);
    // deterministically fill any remainder so no uninitialised bytes are compared
    for k in n..(elemsize - skip) {
        *dst.add(k) = 0xA5;
    }
}

/// Assert two snapshots are identical, with a helpful message.
#[track_caller]
pub fn assert_same<T: PartialEq + std::fmt::Debug>(what: &str, c: &T, rs: &T) {
    if c != rs {
        panic!("DIVERGENCE in {what}\n  C   = {c:?}\n  RUST= {rs:?}");
    }
}

// ---------------------------------------------------------------------------
// MapPair -- drives the same sequence of low-level calls against BOTH .so's
// ---------------------------------------------------------------------------

/// A pair of hash maps (one per library) driven in lock-step.
///
/// Both libraries are always handed *the same* key pointers, so in
/// `STBDS_SH_DEFAULT` mode the stored `char *` is bit-identical and elements can
/// be compared verbatim; in `STDRUP`/`ARENA` mode the key is compared by content.
pub struct MapPair<'a> {
    pub c: *mut c_void,
    pub rs: *mut c_void,
    pub elemsize: usize,
    pub keysize: usize,
    pub mode: c_int,
    /// bytes at element offset 0 that the *library* owns (8 for a stored
    /// `char *`, `keysize` for a raw binary/memcpy key)
    pub skip: usize,
    /// true when `table->string.mode` is one of SH_DEFAULT/STRDUP/ARENA, i.e.
    /// element offset 0 holds a `char *` and `temp_key` is maintained.
    pub stores_ptr: bool,
    pub sh: c_int,
    pub kind: KeyKind,
    pub capi: &'a Api,
    pub rapi: &'a Api,
}

/// How the table's arena mode was established.
#[derive(Clone, Copy, Debug)]
pub enum Arena {
    /// No `stbds_shmode_func`: the table is created lazily by `hmput_key`
    /// (`string.mode` becomes `SH_DEFAULT` iff `mode >= STBDS_HM_STRING`).
    Auto,
    /// Pre-create the table with `stbds_shmode_func(elemsize, m)`.
    Explicit(c_int),
}

impl<'a> MapPair<'a> {
    pub fn new(
        capi: &'a Api,
        rapi: &'a Api,
        elemsize: usize,
        keysize: usize,
        mode: c_int,
        arena: Arena,
    ) -> MapPair<'a> {
        let sh = match arena {
            Arena::Auto => {
                if mode >= HM_STRING {
                    SH_DEFAULT
                } else {
                    SH_NONE
                }
            }
            Arena::Explicit(m) => (m as u32 & 0xff) as c_int,
        };
        let stores_ptr = sh == SH_DEFAULT || sh == SH_STRDUP || sh == SH_ARENA;
        let skip = if stores_ptr { 8 } else { keysize };
        let kind = if sh == SH_STRDUP || sh == SH_ARENA {
            KeyKind::PtrByContent
        } else {
            KeyKind::Binary
        };
        let (c, rs) = match arena {
            Arena::Auto => (std::ptr::null_mut(), std::ptr::null_mut()),
            Arena::Explicit(m) => unsafe {
                ((capi.shmode_func)(elemsize, m), (rapi.shmode_func)(elemsize, m))
            },
        };
        MapPair {
            c,
            rs,
            elemsize,
            keysize,
            mode,
            skip,
            stores_ptr,
            sh,
            kind,
            capi,
            rapi,
        }
    }

    /// `stbds_temp_key` is what `stbds_shputs` reads immediately after a put.
    /// It is compared without ever dereferencing garbage:
    ///
    /// * SH_DEFAULT -- must be *exactly* the caller's key pointer (both
    ///   libraries were handed the same pointer, so this is byte-exact);
    /// * SH_STRDUP / SH_ARENA -- the address differs between libraries, so we
    ///   compare (a) whether it aliases the element's stored key pointer and
    ///   (b) the string content when it does;
    /// * otherwise (`default:` branch) -- the C never writes it, so nothing is
    ///   compared.
    #[track_caller]
    fn check_temp_key(&self, key: *mut c_void, idx: isize) {
        if !self.stores_ptr || self.elemsize < 8 {
            return;
        }
        unsafe {
            let tkc = map_temp_key(self.c, self.elemsize);
            let tkr = map_temp_key(self.rs, self.elemsize);
            let ekc = *((self.c as *const u8).offset(idx * self.elemsize as isize)
                as *const *mut c_char);
            let ekr = *((self.rs as *const u8).offset(idx * self.elemsize as isize)
                as *const *mut c_char);
            if self.sh == SH_DEFAULT {
                assert_same(
                    "SH_DEFAULT temp_key == caller key",
                    &(tkc as usize == key as usize),
                    &(tkr as usize == key as usize),
                );
                assert_same(
                    "SH_DEFAULT elem key == caller key",
                    &(ekc as usize == key as usize),
                    &(ekr as usize == key as usize),
                );
                return;
            }
            let alias_c = tkc as usize == ekc as usize;
            let alias_r = tkr as usize == ekr as usize;
            assert_same("temp_key aliases element key", &alias_c, &alias_r);
            if alias_c {
                assert_same(
                    "temp_key content",
                    &read_cstr(tkc),
                    &read_cstr(tkr),
                );
            }
        }
    }

    /// `hmput_key` on both, then emulate the macro tail `t[temp].value = v`.
    /// Returns the (identical) resolved index.
    #[track_caller]
    pub fn put(&mut self, key: *mut c_void, value: u64) -> isize {
        unsafe {
            let tc = (self.capi.hmput_key)(self.c, self.elemsize, key, self.keysize, self.mode);
            let tr = (self.rapi.hmput_key)(self.rs, self.elemsize, key, self.keysize, self.mode);
            self.c = tc;
            self.rs = tr;
            let ic = map_temp(tc, self.elemsize);
            let ir = map_temp(tr, self.elemsize);
            assert_same("hmput_key temp index", &ic, &ir);
            self.check_temp_key(key, ic);
            let p = value.to_le_bytes();
            write_elem_tail(tc, self.elemsize, ic, self.skip, &p);
            write_elem_tail(tr, self.elemsize, ir, self.skip, &p);
            ic
        }
    }

    /// `hmget_key_ts` on both; returns the (identical) `temp`.
    #[track_caller]
    pub fn get_ts(&mut self, key: *mut c_void) -> isize {
        unsafe {
            let mut tc_i: isize = 0x5AA5;
            let mut tr_i: isize = 0x5AA5;
            let tc = (self.capi.hmget_key_ts)(
                self.c,
                self.elemsize,
                key,
                self.keysize,
                &mut tc_i,
                self.mode,
            );
            let tr = (self.rapi.hmget_key_ts)(
                self.rs,
                self.elemsize,
                key,
                self.keysize,
                &mut tr_i,
                self.mode,
            );
            self.c = tc;
            self.rs = tr;
            assert_same("hmget_key_ts *temp", &tc_i, &tr_i);
            tc_i
        }
    }

    /// `hmget_key` on both; returns the (identical) header `temp`.
    #[track_caller]
    pub fn get(&mut self, key: *mut c_void) -> isize {
        unsafe {
            let tc = (self.capi.hmget_key)(self.c, self.elemsize, key, self.keysize, self.mode);
            let tr = (self.rapi.hmget_key)(self.rs, self.elemsize, key, self.keysize, self.mode);
            self.c = tc;
            self.rs = tr;
            let ic = map_temp(tc, self.elemsize);
            let ir = map_temp(tr, self.elemsize);
            assert_same("hmget_key header temp", &ic, &ir);
            ic
        }
    }

    /// `hmdel_key` on both; returns the (identical) header `temp` (0 = miss, 1 = deleted).
    #[track_caller]
    pub fn del(&mut self, key: *mut c_void, keyoffset: usize) -> isize {
        unsafe {
            let tc = (self.capi.hmdel_key)(
                self.c,
                self.elemsize,
                key,
                self.keysize,
                keyoffset,
                self.mode,
            );
            let tr = (self.rapi.hmdel_key)(
                self.rs,
                self.elemsize,
                key,
                self.keysize,
                keyoffset,
                self.mode,
            );
            assert_same("hmdel_key null-ness", &tc.is_null(), &tr.is_null());
            let (oc, or) = (self.c, self.rs);
            self.c = tc;
            self.rs = tr;
            if tc.is_null() {
                return 0;
            }
            let ic = map_temp(tc, self.elemsize);
            let ir = map_temp(tr, self.elemsize);
            assert_same("hmdel_key header temp", &ic, &ir);
            assert_same(
                "hmdel_key returns-input-pointer",
                &(tc == oc),
                &(tr == or),
            );
            ic
        }
    }

    #[track_caller]
    pub fn check(&self, what: &str) {
        unsafe {
            let sc = snap_map(self.c, self.elemsize, self.kind);
            let sr = snap_map(self.rs, self.elemsize, self.kind);
            assert_same(what, &sc, &sr);
        }
    }

    pub fn snap_c(&self) -> MapSnap {
        unsafe { snap_map(self.c, self.elemsize, self.kind) }
    }

    /// `hmlen` on both.
    #[track_caller]
    pub fn len(&self) -> isize {
        unsafe {
            let a = map_len(self.c, self.elemsize);
            let b = map_len(self.rs, self.elemsize);
            assert_same("hmlen", &a, &b);
            a
        }
    }

    pub fn table_c(&self) -> Option<TableSnap> {
        unsafe { snap_map(self.c, self.elemsize, self.kind).table }
    }

    pub fn free(self) {
        unsafe {
            if !self.c.is_null() {
                (self.capi.hmfree_func)(hash_to_arr(self.c, self.elemsize), self.elemsize);
            }
            if !self.rs.is_null() {
                (self.rapi.hmfree_func)(hash_to_arr(self.rs, self.elemsize), self.elemsize);
            }
        }
    }
}

/// A block of NUL-terminated key strings kept alive for the duration of a test
/// (both libraries receive the very same pointers).
pub struct Keys {
    pub bufs: Vec<Vec<u8>>,
}

impl Keys {
    pub fn random(rng: &mut Rng, n: usize, maxlen: usize) -> Keys {
        let mut bufs = Vec::with_capacity(n);
        let mut seen = std::collections::HashSet::new();
        while bufs.len() < n {
            let len = 1 + rng.below(maxlen);
            let mut s = rng.cstring(len);
            if seen.insert(s.clone()) {
                // 16 bytes of trailing NUL padding so that the
                // `STBDS_SH_NONE` + `STBDS_HM_STRING` path (which does
                // `memcpy(elem, key, keysize)`) never reads past the buffer.
                s.extend_from_slice(&[0u8; 16]);
                bufs.push(s);
            }
        }
        Keys { bufs }
    }
    /// Two **disjoint** key sets (all `n + m` strings are pairwise distinct), so
    /// "absent" keys are guaranteed absent.
    pub fn random_disjoint(rng: &mut Rng, n: usize, m: usize, maxlen: usize) -> (Keys, Keys) {
        let all = Keys::random(rng, n + m, maxlen);
        let mut it = all.bufs.into_iter();
        let a: Vec<Vec<u8>> = it.by_ref().take(n).collect();
        let b: Vec<Vec<u8>> = it.collect();
        (Keys { bufs: a }, Keys { bufs: b })
    }
    pub fn ptr(&self, i: usize) -> *mut c_void {
        self.bufs[i].as_ptr() as *mut c_void
    }
    pub fn cptr(&self, i: usize) -> *mut c_char {
        self.bufs[i].as_ptr() as *mut c_char
    }
    pub fn len(&self) -> usize {
        self.bufs.len()
    }
}

/// Fixed-width binary keys (`keysize` bytes each), distinct.
pub struct BinKeys {
    pub bufs: Vec<Vec<u8>>,
}

impl BinKeys {
    /// `n` buffers of `buflen` bytes that are pairwise distinct **in their
    /// first `prefix` bytes** (that is the only part `memcmp`/`hash_bytes` sees
    /// for `keysize == prefix`).  `prefix == 0` means all keys compare equal, so
    /// distinctness is impossible and a single repeated buffer is returned.
    pub fn random_prefix(rng: &mut Rng, n: usize, prefix: usize, buflen: usize) -> BinKeys {
        let buflen = buflen.max(prefix).max(1);
        if prefix == 0 {
            return BinKeys {
                bufs: (0..n.max(1)).map(|_| rng.bytes(buflen)).collect(),
            };
        }
        let mut bufs: Vec<Vec<u8>> = Vec::with_capacity(n);
        let mut seen = std::collections::HashSet::new();
        let mut guard = 0usize;
        while bufs.len() < n {
            guard += 1;
            assert!(
                guard < 1000 * n + 100_000,
                "cannot make {n} keys distinct over {prefix} bytes"
            );
            let k = rng.bytes(buflen);
            if seen.insert(k[..prefix].to_vec()) {
                bufs.push(k);
            }
        }
        BinKeys { bufs }
    }

    pub fn random(rng: &mut Rng, n: usize, keysize: usize) -> BinKeys {
        BinKeys::random_prefix(rng, n, keysize, keysize.max(8) + 8)
    }

    /// How many pairwise-distinct keys a `keysize`-byte key space can hold
    /// (capped so the retry loop stays fast).
    pub fn max_distinct(keysize: usize) -> usize {
        match keysize {
            0 => 1,
            1 => 180,
            2 => 40_000,
            _ => usize::MAX,
        }
    }
    /// Two **disjoint** key sets, pairwise distinct over their first `prefix`
    /// bytes.
    pub fn random_disjoint(
        rng: &mut Rng,
        n: usize,
        m: usize,
        prefix: usize,
        buflen: usize,
    ) -> (BinKeys, BinKeys) {
        let all = BinKeys::random_prefix(rng, n + m, prefix, buflen);
        let mut it = all.bufs.into_iter();
        let a: Vec<Vec<u8>> = it.by_ref().take(n).collect();
        let b: Vec<Vec<u8>> = it.collect();
        (BinKeys { bufs: a }, BinKeys { bufs: b })
    }
    pub fn ptr(&self, i: usize) -> *mut c_void {
        self.bufs[i].as_ptr() as *mut c_void
    }
    pub fn len(&self) -> usize {
        self.bufs.len()
    }
}

//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries (the C reference and the Rust translation)
//! through `libloading` and exposes the 16 exported symbols behind identical
//! function-pointer types.  No Rust function is ever called directly — every
//! call goes through `dlsym`, exactly like an external consumer.

#![allow(dead_code)]
#![allow(non_snake_case)]

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// layout-compatible mirrors of the C structs (for introspection only)
// ---------------------------------------------------------------------------

pub const HDRSIZE: usize = 32;
pub const BUCKET_LENGTH: usize = 8;
pub const BUCKET_SHIFT: usize = 3;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StringArena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

impl StringArena {
    pub fn new() -> Self {
        StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 }
    }
}

#[repr(C)]
pub struct StringBlock {
    pub next: *mut StringBlock,
    pub storage: [c_char; 8],
}

#[repr(C)]
pub struct HashBucket {
    pub hash: [usize; BUCKET_LENGTH],
    pub index: [isize; BUCKET_LENGTH],
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

const _: () = assert!(std::mem::size_of::<ArrayHeader>() == 32);
const _: () = assert!(std::mem::size_of::<HashBucket>() == 128);
const _: () = assert!(std::mem::size_of::<HashIndex>() == 104);
const _: () = assert!(std::mem::size_of::<StringArena>() == 24);

pub const STBDS_HM_BINARY: c_int = 0;
pub const STBDS_HM_STRING: c_int = 1;

pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

// ---------------------------------------------------------------------------
// function-pointer types
// ---------------------------------------------------------------------------

pub type FnArrGrowF = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrFreeF = unsafe extern "C" fn(*mut c_void);
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnStrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
pub type FnStrReset = unsafe extern "C" fn(*mut StringArena);
pub type FnHmFree = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmGetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type FnHmGetKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmPutKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnShModeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnHmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnShGeti = unsafe extern "C" fn(c_int);
pub type FnStrKey = unsafe extern "C" fn(c_int) -> *mut c_char;

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
    pub hmget_key_ts: FnHmGetKeyTs,
    pub hmget_key: FnHmGetKey,
    pub hmput_default: FnHmPutDefault,
    pub hmput_key: FnHmPutKey,
    pub shmode_func: FnShModeFunc,
    pub hmdel_key: FnHmDelKey,
    pub sh_geti: FnShGeti,
    pub strkey: FnStrKey,
}

impl Lib {
    unsafe fn load(name: &'static str, path: &PathBuf) -> Lib {
        let lib = libloading::Library::new(path)
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
        macro_rules! g {
            ($t:ty, $s:expr) => {{
                let sym: libloading::Symbol<$t> = lib
                    .get($s)
                    .unwrap_or_else(|e| panic!("{} missing symbol {:?}: {e}", name, $s));
                *sym
            }};
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
            hmget_key_ts: g!(FnHmGetKeyTs, b"stbds_hmget_key_ts\0"),
            hmget_key: g!(FnHmGetKey, b"stbds_hmget_key\0"),
            hmput_default: g!(FnHmPutDefault, b"stbds_hmput_default\0"),
            hmput_key: g!(FnHmPutKey, b"stbds_hmput_key\0"),
            shmode_func: g!(FnShModeFunc, b"stbds_shmode_func\0"),
            hmdel_key: g!(FnHmDelKey, b"stbds_hmdel_key\0"),
            sh_geti: g!(FnShGeti, b"sh_geti\0"),
            strkey: g!(FnStrKey, b"strkey\0"),
            _lib: lib,
        }
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src/build");
    let mut found: Option<PathBuf> = None;
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|s| s == "so").unwrap_or(false) {
                found = Some(p);
                break;
            }
        }
    }
    found.unwrap_or_else(|| panic!("no .so found under {}; build the C library first", build.display()))
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for prof in ["release", "debug"] {
        let p = root.join("target").join(prof).join("libsh_geti_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libsh_geti_lib.so not found; run `cargo build --release` first");
}

pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

/// Both libraries, loaded once per test process.
pub fn pair() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| unsafe {
        Pair { c: Lib::load("C", &find_c_so()), r: Lib::load("RUST", &find_rust_so()) }
    })
}

/// Serialises tests.  Both libraries keep *process-global* mutable state
/// (`stbds_hash_seed`, the `static char buffer[256]` behind `strkey`) and
/// `capture_stdout` redirects fd 1 process-wide, so tests must not overlap.
pub fn serial() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::{Mutex, OnceLock};
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    let m = L.get_or_init(|| Mutex::new(()));
    m.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// deterministic RNG (SplitMix64) -- fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const TEST_SEED: u64 = 0x5eed_1234_abcd_0001;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| (self.next_u64() & 0xff) as u8).collect()
    }
    /// Random printable-ASCII C string of `n` bytes (no NUL inside).
    pub fn cstring(&mut self, n: usize) -> CString {
        let v: Vec<u8> = (0..n).map(|_| b'a' + (self.next_u64() % 26) as u8).collect();
        CString::new(v).unwrap()
    }
}

// ---------------------------------------------------------------------------
// table serialisation
//
// We must NOT compare raw pointers (the two libraries allocate independently),
// and we must NOT compare bytes the C code never initialises (realloc'ed
// memory).  Everything below is derived from fields the C code always writes.
// ---------------------------------------------------------------------------

/// How the first `keysize` bytes of an element should be rendered.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyRepr {
    /// key bytes stored inline (BINARY mode, or `string.mode == SH_NONE`)
    Raw,
    /// key is a `char *` (string.mode DEFAULT / STRDUP / ARENA)
    Ptr,
}

pub unsafe fn hdr_of(handle: *mut c_void, elemsize: usize) -> *mut ArrayHeader {
    (handle as *mut u8).sub(elemsize).sub(HDRSIZE) as *mut ArrayHeader
}

unsafe fn cstr_dump(p: *const c_char, out: &mut String) {
    if p.is_null() {
        out.push_str("<null>");
    } else {
        let s = CStr::from_ptr(p).to_bytes();
        out.push('"');
        for b in s {
            if b.is_ascii_graphic() || *b == b' ' {
                out.push(*b as char);
            } else {
                out.push_str(&format!("\\x{:02x}", b));
            }
        }
        out.push('"');
    }
}

/// Serialise everything observable about a hash-map handle.
///
/// `handle` is the value returned by `stbds_hmput_key` & friends (i.e. it
/// points at element 0 of the array, one `elemsize` past the raw array base).
pub unsafe fn dump_table(handle: *mut c_void, elemsize: usize, keysize: usize) -> String {
    let mut out = String::new();
    if handle.is_null() {
        out.push_str("handle=NULL\n");
        return out;
    }
    let h = &*hdr_of(handle, elemsize);
    out.push_str(&format!("length={} capacity={} temp={}\n", h.length, h.capacity, h.temp));

    let table = h.hash_table as *mut HashIndex;
    // The C `switch (table->string.mode)` stores a `char *` only for
    // SH_DEFAULT(1) / SH_STRDUP(2) / SH_ARENA(3); every other value (including
    // out-of-range ones truncated into the `unsigned char` field) falls into
    // `default:` and memcpy's raw key bytes.
    let repr = if table.is_null() {
        KeyRepr::Raw
    } else if (1..=3).contains(&(*table).string.mode) {
        KeyRepr::Ptr
    } else {
        KeyRepr::Raw
    };

    // elements: index 0 is the "default" slot, 1.. are live entries
    let raw = (handle as *mut u8).sub(elemsize);
    for i in 0..h.length {
        let e = raw.add(elemsize * i);
        out.push_str(&format!("  e[{}] ", i));
        if repr == KeyRepr::Ptr && elemsize >= 8 {
            cstr_dump(*(e as *const *const c_char), &mut out);
            out.push(' ');
            for b in std::slice::from_raw_parts(e.add(8), elemsize - 8) {
                out.push_str(&format!("{:02x}", b));
            }
        } else {
            for b in std::slice::from_raw_parts(e, elemsize) {
                out.push_str(&format!("{:02x}", b));
            }
        }
        out.push('\n');
    }
    let _ = keysize;

    if table.is_null() {
        out.push_str("table=NULL\n");
        return out;
    }
    let t = &*table;
    out.push_str(&format!(
        "table slot_count={} used={} used_thr={} shrink_thr={} tomb={} tomb_thr={} seed={:#x} log2={}\n",
        t.slot_count,
        t.used_count,
        t.used_count_threshold,
        t.used_count_shrink_threshold,
        t.tombstone_count,
        t.tombstone_count_threshold,
        t.seed,
        t.slot_count_log2
    ));
    out.push_str(&format!(
        "arena remaining={} block={} mode={} storage_null={}\n",
        t.string.remaining,
        t.string.block,
        t.string.mode,
        t.string.storage.is_null()
    ));
    let nbuckets = t.slot_count >> BUCKET_SHIFT;
    for bi in 0..nbuckets {
        let b = &*t.storage.add(bi);
        out.push_str(&format!("  b[{}]", bi));
        for j in 0..BUCKET_LENGTH {
            out.push_str(&format!(" {:#x}/{}", b.hash[j], b.index[j]));
        }
        out.push('\n');
    }
    out
}

/// Serialise a `stbds_string_arena` (pointer-free).
pub unsafe fn dump_arena(a: *const StringArena) -> String {
    let a = &*a;
    let mut nblocks = 0usize;
    let mut x = a.storage as *const StringBlock;
    while !x.is_null() {
        nblocks += 1;
        if nblocks > 10_000 {
            break;
        }
        x = (*x).next;
    }
    format!(
        "arena remaining={} block={} mode={} storage_null={} nblocks={}",
        a.remaining,
        a.block,
        a.mode,
        a.storage.is_null(),
        nblocks
    )
}

// ---------------------------------------------------------------------------
// stdout capture (for sh_geti, which printf()s)
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

const O_WRONLY: c_int = 1;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;

/// Redirect fd 1 to a temporary file, run `f`, restore, and return the bytes
/// written.  Works for `printf` inside a `dlopen`ed library because both
/// libraries share the process's libc `stdout`.
pub fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let path = std::env::temp_dir().join(format!(
        "shgeti_cap_{}_{}_{}.txt",
        std::process::id(),
        tag,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let cpath = CString::new(path.to_str().unwrap()).unwrap();
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let fd = open(cpath.as_ptr(), O_WRONLY | O_CREAT | O_TRUNC, 0o644 as c_int);
        assert!(fd >= 0, "open({}) failed", path.display());
        assert!(dup2(fd, 1) >= 0, "dup2 failed");
        close(fd);

        f();

        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
    }
    let data = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    data
}

// ---------------------------------------------------------------------------
// small helpers
// ---------------------------------------------------------------------------

/// A heap buffer whose contents can be handed to the libraries as a key.
pub fn boxed_bytes(v: &[u8]) -> Box<[u8]> {
    v.to_vec().into_boxed_slice()
}

/// `hdr(handle - elemsize)->temp`
pub unsafe fn temp_of(h: *mut c_void, elemsize: usize) -> isize {    (*((h as *mut u8).sub(elemsize).sub(HDRSIZE) as *mut ArrayHeader)).temp
}

/// `hdr(handle - elemsize)`
pub unsafe fn header_of(h: *mut c_void, elemsize: usize) -> &'static ArrayHeader {
    &*((h as *mut u8).sub(elemsize).sub(HDRSIZE) as *mut ArrayHeader)
}

/// `hdr(handle - elemsize)->hash_table`
pub unsafe fn table_of(h: *mut c_void, elemsize: usize) -> *mut HashIndex {
    header_of(h, elemsize).hash_table as *mut HashIndex
}

/// `stbds_hmput_key` followed by the value store the stb macros always do
/// (`t[stbds_temp(t-1)].value = v`).  Without this the bytes between `keysize`
/// and `elemsize` stay as whatever `realloc` returned, which differs between
/// the two libraries for reasons that have nothing to do with the translation.
pub unsafe fn put_and_fill(
    lib: &Lib,
    h: *mut c_void,
    elemsize: usize,
    keysize: usize,
    key: *mut c_void,
    mode: c_int,
    fill: u8,
) -> (*mut c_void, isize) {
    let h = (lib.hmput_key)(h, elemsize, key, keysize, mode);
    let idx = temp_of(h, elemsize);
    if keysize < elemsize {
        let e = (h as *mut u8).offset(idx * elemsize as isize);
        for b in keysize..elemsize {
            *e.add(b) = fill.wrapping_add(b as u8);
        }
    }
    (h, idx)
}

/// `stbds_hmfree_func` on a handle returned by the hm/sh entry points.
pub unsafe fn free_handle(lib: &Lib, h: *mut c_void, elemsize: usize) {
    if !h.is_null() {
        (lib.hmfree_func)((h as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

pub fn assert_eq_dump(what: &str, c: &str, r: &str) {    if c != r {
        let mut msg = format!("DIVERGENCE in {what}\n--- C ---\n{c}\n--- RUST ---\n{r}\n--- first differing line ---\n");
        for (i, (a, b)) in c.lines().zip(r.lines()).enumerate() {
            if a != b {
                msg.push_str(&format!("line {i}:\n  C: {a}\n  R: {b}\n"));
                break;
            }
        }
        panic!("{msg}");
    }
}

// ---------------------------------------------------------------------------
// forked-child execution
//
// Several rows of ERRORS.md are conditions under which the C *crashes* or
// `assert()`-aborts.  "Both libraries die the same way" is a real, comparable
// result, but it cannot be observed in-process.  We therefore run the call in a
// forked child and compare the wait status.
// ---------------------------------------------------------------------------

extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

/// Runs `f` in a forked child with stdout/stderr sent to /dev/null and returns
/// a description of how the child terminated.
pub fn child_outcome<F: FnOnce()>(f: F) -> String {
    let devnull = CString::new("/dev/null").unwrap();
    unsafe {
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            let fd = open(devnull.as_ptr(), O_WRONLY, 0o644 as c_int);
            if fd >= 0 {
                dup2(fd, 1);
                dup2(fd, 2);
                close(fd);
            }
            f();
            _exit(0);
        }
        let mut st: c_int = 0;
        let w = waitpid(pid, &mut st, 0);
        assert_eq!(w, pid, "waitpid failed");
        let sig = st & 0x7f;
        if sig == 0 {
            format!("exited({})", (st >> 8) & 0xff)
        } else {
            format!("signal({})", sig)
        }
    }
}

pub const SIGABRT: &str = "signal(6)";

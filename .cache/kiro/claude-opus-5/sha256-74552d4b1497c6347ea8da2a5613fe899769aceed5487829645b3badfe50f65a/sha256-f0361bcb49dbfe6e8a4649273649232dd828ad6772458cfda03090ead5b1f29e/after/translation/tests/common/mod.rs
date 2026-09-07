//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` through `libloading` and calls
//! every entry point through its exported symbol only — never through the Rust
//! crate directly — so the `#[no_mangle]` wrappers are part of what is tested.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Layout-compatible mirrors of the C structures (x86-64 SysV)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct StringArena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HashBucket {
    pub hash: [usize; 8],
    pub index: [isize; 8],
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
    pub string: StringArena,
    pub storage: *mut HashBucket,
}

pub const HDR_SIZE: usize = std::mem::size_of::<ArrayHeader>();

pub const STBDS_HM_BINARY: c_int = 0;
pub const STBDS_HM_STRING: c_int = 1;

pub const STBDS_SH_NONE: c_int = 0;
pub const STBDS_SH_DEFAULT: c_int = 1;
pub const STBDS_SH_STRDUP: c_int = 2;
pub const STBDS_SH_ARENA: c_int = 3;

// ---------------------------------------------------------------------------
// Function-pointer types
// ---------------------------------------------------------------------------

pub type FnArrGrowF = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrFreeF = unsafe extern "C" fn(*mut c_void);
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
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
pub type FnStrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
pub type FnStrReset = unsafe extern "C" fn(*mut StringArena);
pub type FnStrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnHelxo = unsafe extern "C" fn(c_char);

/// Every exported symbol of one implementation.
pub struct Api {
    pub name: &'static str,
    _lib: libloading::Library,
    pub arrgrowf: FnArrGrowF,
    pub arrfreef: FnArrFreeF,
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
    pub helxo: FnHelxo,
}

macro_rules! sym {
    ($lib:expr, $n:literal, $t:ty) => {{
        let s: libloading::Symbol<$t> = $lib
            .get(concat!($n, "\0").as_bytes())
            .unwrap_or_else(|e| panic!("missing symbol {}: {}", $n, e));
        *s
    }};
}

impl Api {
    pub unsafe fn load(name: &'static str, path: &PathBuf) -> Api {
        let lib = libloading::Library::new(path)
            .unwrap_or_else(|e| panic!("cannot load {}: {}", path.display(), e));
        let api = Api {
            name,
            arrgrowf: sym!(lib, "stbds_arrgrowf", FnArrGrowF),
            arrfreef: sym!(lib, "stbds_arrfreef", FnArrFreeF),
            rand_seed: sym!(lib, "stbds_rand_seed", FnRandSeed),
            hash_string: sym!(lib, "stbds_hash_string", FnHashString),
            hash_bytes: sym!(lib, "stbds_hash_bytes", FnHashBytes),
            hmfree_func: sym!(lib, "stbds_hmfree_func", FnHmFree),
            hmget_key_ts: sym!(lib, "stbds_hmget_key_ts", FnHmGetKeyTs),
            hmget_key: sym!(lib, "stbds_hmget_key", FnHmGetKey),
            hmput_default: sym!(lib, "stbds_hmput_default", FnHmPutDefault),
            hmput_key: sym!(lib, "stbds_hmput_key", FnHmPutKey),
            shmode_func: sym!(lib, "stbds_shmode_func", FnShModeFunc),
            hmdel_key: sym!(lib, "stbds_hmdel_key", FnHmDelKey),
            stralloc: sym!(lib, "stbds_stralloc", FnStrAlloc),
            strreset: sym!(lib, "stbds_strreset", FnStrReset),
            strkey: sym!(lib, "strkey", FnStrKey),
            helxo: sym!(lib, "helxo", FnHelxo),
            _lib: lib,
        };
        api
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has a parent directory")
        .to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    let dir = workspace_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            let n = p.file_name().unwrap().to_string_lossy().to_string();
            if n.starts_with("lib") && n.ends_with(".so") {
                found.push(p);
            }
        }
    }
    assert_eq!(
        found.len(),
        1,
        "expected exactly one C .so in {}, found {:?}",
        dir.display(),
        found
    );
    found.pop().unwrap()
}

pub fn rust_so_path() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = root.join("src/lib.rs");
    let src_m = std::fs::metadata(&src).and_then(|m| m.modified()).ok();

    // explicit override, used by run_all.sh to pin a specific profile
    let candidates: Vec<PathBuf> = match std::env::var("HARVEST_RUST_SO") {
        Ok(p) => vec![PathBuf::from(p)],
        Err(_) => vec![
            root.join("target/release/libhelxo_lib.so"),
            root.join("target/debug/libhelxo_lib.so"),
        ],
    };

    let mut best: Option<(PathBuf, std::time::SystemTime)> = None;
    for c in candidates {
        if let Ok(m) = std::fs::metadata(&c).and_then(|m| m.modified()) {
            if best.as_ref().map(|(_, t)| m > *t).unwrap_or(true) {
                best = Some((c, m));
            }
        }
    }
    let (p, m) = best.expect(
        "no Rust cdylib found; run `cargo build --release` in translation/ before `cargo test`",
    );
    if let Some(sm) = src_m {
        assert!(
            m >= sm,
            "stale Rust .so at {} (older than src/lib.rs); run `cargo build --release`",
            p.display()
        );
    }
    p
}

pub struct Pair {
    pub c: Api,
    pub r: Api,
}

pub fn load_pair() -> Pair {
    unsafe {
        Pair {
            c: Api::load("C", &c_so_path()),
            r: Api::load("RUST", &rust_so_path()),
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xoshiro-ish; fixed seed for reproducibility)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E3779B97F4A7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % n
        }
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 13) as u8
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
    /// NUL-terminated C string of `n` non-NUL bytes drawn from 1..=255.
    pub fn cstring(&mut self, n: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..n).map(|_| 1 + (self.next_u64() % 255) as u8).collect();
        v.push(0);
        v
    }
    /// NUL-terminated printable C string (safe for `%s`-style inspection).
    pub fn ascii_cstring(&mut self, n: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..n)
            .map(|_| b'a' + (self.next_u64() % 26) as u8)
            .collect();
        v.push(0);
        v
    }
}

// ---------------------------------------------------------------------------
// Structure dumping — canonical, pointer-value-independent
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyKind {
    /// element bytes are raw data; compare them verbatim
    Binary,
    /// the first 8 bytes of each element are a `char *`; compare the pointee string
    StringPtr,
}

pub unsafe fn header_of_arr(a: *mut c_void) -> *mut ArrayHeader {
    (a as *mut u8).sub(HDR_SIZE) as *mut ArrayHeader
}

pub unsafe fn header_of_hash(h: *mut c_void, elemsize: usize) -> *mut ArrayHeader {
    header_of_arr((h as *mut u8).sub(elemsize) as *mut c_void)
}

pub unsafe fn cstr_repr(p: *const c_char) -> String {
    if p.is_null() {
        return "<null>".to_string();
    }
    let mut out = String::new();
    let mut q = p as *const u8;
    let mut n = 0usize;
    while *q != 0 && n < 4096 {
        out.push_str(&format!("{:02x}", *q));
        q = q.add(1);
        n += 1;
    }
    format!("str[{}]:{}", n, out)
}

pub fn hexdump(p: *const u8, n: usize) -> String {
    let mut s = String::with_capacity(n * 2);
    for i in 0..n {
        s.push_str(&format!("{:02x}", unsafe { *p.add(i) }));
    }
    s
}

/// Dump a plain (non-hash) array: header + `len*elemsize` bytes.
pub unsafe fn dump_arr(a: *mut c_void, elemsize: usize) -> String {
    if a.is_null() {
        return "ARR:<null>".to_string();
    }
    let h = header_of_arr(a);
    let mut s = format!(
        "ARR len={} cap={} temp={} ht={}",
        (*h).length,
        (*h).capacity,
        (*h).temp,
        if (*h).hash_table.is_null() { "0" } else { "!0" }
    );
    let n = (*h).length.saturating_mul(elemsize);
    if n <= 65536 {
        s.push_str(&format!(" data={}", hexdump(a as *const u8, n)));
    }
    s
}

/// Dump a hash-map pointer (the value returned by `stbds_hmput_key` &c.).
pub unsafe fn dump_hash(h: *mut c_void, elemsize: usize, kind: KeyKind) -> String {
    if h.is_null() {
        return "HASH:<null>".to_string();
    }
    let arr = (h as *mut u8).sub(elemsize) as *mut c_void;
    let hdr = header_of_arr(arr);
    let mut s = format!(
        "HASH len={} cap={} temp={}",
        (*hdr).length,
        (*hdr).capacity,
        (*hdr).temp
    );

    let t = (*hdr).hash_table as *mut HashIndex;
    if t.is_null() {
        s.push_str(" table=<null>");
    } else {
        s.push_str(&format!(
            " table{{slot_count={} used={} used_thr={} shrink_thr={} tomb={} tomb_thr={} seed={:#x} log2={} arena{{rem={} block={} mode={} storage={}}} temp_key={}}}",
            (*t).slot_count,
            (*t).used_count,
            (*t).used_count_threshold,
            (*t).used_count_shrink_threshold,
            (*t).tombstone_count,
            (*t).tombstone_count_threshold,
            (*t).seed,
            (*t).slot_count_log2,
            (*t).string.remaining,
            (*t).string.block,
            (*t).string.mode,
            if (*t).string.storage.is_null() { "0" } else { "!0" },
            // NOTE: `temp_key` is left UNINITIALISED by `stbds_make_hash_index`
            // (it is only ever written by the `stbds_temp_key` macro), so it
            // must never be dereferenced here.  Tests that care about its value
            // use `temp_key_str()` right after a string-mode put, where the C
            // guarantees it points at a live key.
            if (*t).temp_key.is_null() { "0" } else { "!0" },
        ));
        s.push_str(" buckets=[");
        let nb = (*t).slot_count >> 3;
        for b in 0..nb {
            let bk = (*t).storage.add(b);
            for j in 0..8 {
                s.push_str(&format!("({:#x},{})", (*bk).hash[j], (*bk).index[j]));
            }
            s.push('|');
        }
        s.push(']');
    }

    s.push_str(" elems=[");
    let len = (*hdr).length;
    for i in 0..len.min(4096) {
        let e = (arr as *mut u8).add(i * elemsize);
        match kind {
            KeyKind::Binary => s.push_str(&hexdump(e as *const u8, elemsize)),
            KeyKind::StringPtr => {
                let kp = *(e as *mut *mut c_char);
                s.push_str(&cstr_repr(kp));
                if elemsize > 8 {
                    // Only the bytes the library actually writes are compared;
                    // padding after `value` is uninitialised in both builds.
                    s.push_str(&format!("+{}", hexdump(e.add(8) as *const u8, 1)));
                }
            }
        }
        s.push(',');
    }
    s.push(']');
    s
}

/// Contents of `table->temp_key`. Only valid immediately after a string-mode
/// `stbds_hmput_key` (the only place the C writes it).
pub unsafe fn temp_key_str(h: *mut c_void, elemsize: usize) -> String {
    let arr = (h as *mut u8).sub(elemsize) as *mut c_void;
    let hdr = header_of_arr(arr);
    let t = (*hdr).hash_table as *mut HashIndex;
    if t.is_null() {
        return "<no table>".to_string();
    }
    cstr_repr((*t).temp_key)
}

pub unsafe fn dump_arena(a: *const StringArena) -> String {
    format!(
        "ARENA{{rem={} block={} mode={} storage={}}}",
        (*a).remaining,
        (*a).block,
        (*a).mode,
        if (*a).storage.is_null() { "0" } else { "!0" }
    )
}

// ---------------------------------------------------------------------------
// stdout capture (for `helxo`)
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

const O_RDWR: c_int = 2;
const O_CREAT: c_int = 64;
const O_TRUNC: c_int = 512;

/// Run `f` with fd 1 redirected to a temp file and return the bytes written.
pub fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let path = std::env::temp_dir().join(format!(
        "helxo_cap_{}_{}.txt",
        std::process::id(),
        tag.replace('/', "_")
    ));
    let cpath = std::ffi::CString::new(path.to_string_lossy().as_bytes()).unwrap();
    unsafe {
        let fd = open(cpath.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int);
        assert!(fd >= 0, "open temp file failed");
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0);
        assert!(dup2(fd, 1) >= 0);
        f();
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0);
        close(saved);
        close(fd);
    }
    let data = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    data
}

// ---------------------------------------------------------------------------
// Assertion helper
// ---------------------------------------------------------------------------

pub fn diff(ctx: &str, c: &str, r: &str) {
    if c != r {
        let common = c
            .bytes()
            .zip(r.bytes())
            .take_while(|(a, b)| a == b)
            .count();
        let lo = common.saturating_sub(60);
        panic!(
            "DIVERGENCE in {}\n  first difference at byte {}\n  C   ...{}\n  RUST...{}\n",
            ctx,
            common,
            &c[lo..(common + 120).min(c.len())],
            &r[lo..(common + 120).min(r.len())],
        );
    }
}

/// Zero the part of the most-recently-touched element that the library leaves
/// indeterminate.
///
/// `stbds_hmput_key` only defines the first `lib_writes` bytes of a new element
/// (`keysize` for the `default:` memcpy arm, 8 for the three pointer-storing
/// arena arms).  Everything past that is whatever `realloc` handed back, so a
/// differential dump of the raw element bytes must be normalised first — the C
/// and the Rust get their memory from independent allocation histories.
pub unsafe fn normalize_last_elem(t: *mut c_void, elemsize: usize, lib_writes: usize) {
    if t.is_null() || elemsize <= lib_writes {
        return;
    }
    let idx = (*header_of_hash(t, elemsize)).temp;
    if idx < 0 {
        return;
    }
    let e = (t as *mut u8).offset(idx * elemsize as isize);
    std::ptr::write_bytes(e.add(lib_writes), 0, elemsize - lib_writes);
}

/// Arena `block` values whose implied `blocksize` (`512 << ((block>>1) & 63)`)
/// is either 0 (wrapped) or small enough to actually allocate.  Larger ones make
/// the C's own `realloc` fail and then dereference NULL, which is UB in both
/// implementations and so not differentially observable.
pub fn arena_block_is_testable(block: u8) -> bool {
    let bs = 512usize.wrapping_shl(((block >> 1) as u32) & 63);
    bs == 0 || bs <= (1usize << 21)
}
